use anyhow::Result;
use indexmap::{IndexMap, IndexSet};
use next_core::{
    next_app::AppEntry,
    next_manifests::{
        WebpackStats, WebpackStatsAsset, WebpackStatsChunk, WebpackStatsEntrypoint,
        WebpackStatsEntrypointAssets, WebpackStatsModule,
    },
};
use turbo_tasks::{RcStr, Vc};
use turbopack_browser::ecmascript::EcmascriptDevChunk;
use turbopack_core::{
    chunk::{Chunk, ChunkItem},
    output::OutputAsset,
};

fn normalize_client_path(path: &str) -> String {
    let next_re = regex::Regex::new(r"^_next/").unwrap();
    next_re.replace(path, ".next/").into()
}

pub async fn generate_webpack_stats(
    entrypoint: &AppEntry,
    entry_assets: &IndexSet<Vc<Box<dyn OutputAsset>>>,
) -> Result<WebpackStats> {
    let mut assets = vec![];
    let mut chunks = vec![];
    let mut chunk_items: IndexMap<Vc<Box<dyn ChunkItem>>, IndexSet<RcStr>> = IndexMap::new();
    let mut modules = vec![];
    for asset in entry_assets {
        let asset = *asset;
        let path = normalize_client_path(&asset.ident().path().await?.path);

        let Some(asset_len) = *asset.len().await? else {
            continue;
        };

        let asset_len = *asset_len.await?;
        if let Some(chunk) = Vc::try_resolve_downcast_type::<EcmascriptDevChunk>(asset).await? {
            let chunk_ident = normalize_client_path(&chunk.ident().path().await?.path);
            chunks.push(WebpackStatsChunk {
                size: asset_len,
                files: vec![chunk_ident.clone().into()],
                id: chunk_ident.clone().into(),
                ..Default::default()
            });

            for item in chunk.chunk().chunk_items().await? {
                // let name =
                chunk_items
                    .entry(*item)
                    .or_default()
                    .insert(chunk_ident.clone().into());
            }
        }

        assets.push(WebpackStatsAsset {
            ty: "asset".into(),
            name: path.clone().into(),
            chunks: vec![path.into()],
            size: asset_len,
            ..Default::default()
        });
    }

    for (chunk_item, chunks) in chunk_items {
        let size = match &*chunk_item.content_ident().path().read().len().await? {
            Some(size) => Some(*size.await?),
            None => None,
        };
        let path = chunk_item.asset_ident().path().await?.path.clone();
        modules.push(WebpackStatsModule {
            name: path.clone(),
            id: path.clone(),
            chunks: chunks.into_iter().collect(),
            size,
        });
    }

    let mut entrypoints = IndexMap::new();
    entrypoints.insert(
        entrypoint.original_name.clone(),
        WebpackStatsEntrypoint {
            name: entrypoint.original_name.clone(),
            chunks: chunks.iter().map(|c| c.id.clone()).collect(),
            assets: assets
                .iter()
                .map(|a| WebpackStatsEntrypointAssets {
                    name: a.name.clone(),
                })
                .collect(),
        },
    );

    Ok(WebpackStats {
        assets,
        entrypoints,
        chunks,
        modules,
    })
}
