use std::path::MAIN_SEPARATOR;

use anyhow::Result;
use indexmap::IndexMap;
use indoc::formatdoc;
use turbo_tasks::{RcStr, Value, ValueToString, Vc};
use turbo_tasks_fs::FileSystemPath;
use turbopack::{transition::Transition, ModuleAssetContext};
use turbopack_core::{
    file_source::FileSource,
    module::Module,
    reference_type::{EcmaScriptModulesReferenceSubType, ReferenceType},
    source::Source,
};
use turbopack_ecmascript::{magic_identifier, utils::StringifyJs};

pub struct BaseLoaderTreeBuilder {
    pub inner_assets: IndexMap<RcStr, Vc<Box<dyn Module>>>,
    counter: usize,
    pub imports: Vec<RcStr>,
    pub module_asset_context: Vc<ModuleAssetContext>,
    pub server_component_transition: Vc<Box<dyn Transition>>,
    project_root: Vc<FileSystemPath>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppDirModuleType {
    Page,
    DefaultPage,
    Error,
    Layout,
    Loading,
    Template,
    NotFound,
    Interceptor,
}

impl AppDirModuleType {
    pub fn name(&self) -> &'static str {
        match self {
            AppDirModuleType::Page => "page",
            AppDirModuleType::DefaultPage => "defaultPage",
            AppDirModuleType::Error => "error",
            AppDirModuleType::Layout => "layout",
            AppDirModuleType::Loading => "loading",
            AppDirModuleType::Template => "template",
            AppDirModuleType::NotFound => "not-found",
            AppDirModuleType::Interceptor => "interceptor",
        }
    }
}

impl BaseLoaderTreeBuilder {
    pub fn new(
        module_asset_context: Vc<ModuleAssetContext>,
        server_component_transition: Vc<Box<dyn Transition>>,
        project_root: Vc<FileSystemPath>,
    ) -> Self {
        BaseLoaderTreeBuilder {
            inner_assets: IndexMap::new(),
            counter: 0,
            imports: Vec::new(),
            module_asset_context,
            server_component_transition,
            project_root,
        }
    }

    pub fn unique_number(&mut self) -> usize {
        let i = self.counter;
        self.counter += 1;
        i
    }

    pub fn process_source(&self, source: Vc<Box<dyn Source>>) -> Vc<Box<dyn Module>> {
        let reference_type = Value::new(ReferenceType::EcmaScriptModules(
            EcmaScriptModulesReferenceSubType::Undefined,
        ));

        self.server_component_transition
            .process(source, self.module_asset_context, reference_type)
            .module()
    }

    pub fn process_module(&self, module: Vc<Box<dyn Module>>) -> Vc<Box<dyn Module>> {
        self.server_component_transition
            .process_module(module, self.module_asset_context)
    }

    pub async fn create_module_tuple_code(
        &mut self,
        module_type: AppDirModuleType,
        path: Vc<FileSystemPath>,
    ) -> Result<String> {
        let name = module_type.name();
        let i = self.unique_number();
        let identifier = magic_identifier::mangle(&format!("{name} #{i}"));

        self.imports.push(
            formatdoc!(
                r#"
                import * as {} from "MODULE_{}";
                "#,
                identifier,
                i
            )
            .into(),
        );

        let module = self.process_source(Vc::upcast(FileSource::new(path)));

        self.inner_assets
            .insert(format!("MODULE_{i}").into(), module);

        let module_path = module.ident().path().to_string().await?;

        let project_root_with_trailing_sep =
            format!("{}{}", self.project_root.to_string().await?, MAIN_SEPARATOR);

        let relative_path = module_path
            .strip_prefix(&project_root_with_trailing_sep)
            .unwrap_or(module_path.as_str());

        Ok(format!(
            "[() => {identifier}, {path}, {relative_path}]",
            path = StringifyJs(&module_path),
            relative_path = StringifyJs(&relative_path),
        ))
    }
}
