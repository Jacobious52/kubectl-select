
use clipboard::{ClipboardContext, ClipboardProvider};

use crate::kubectl::kubectl_base_cmd;

pub struct BindingContext {
    pub namespace: Option<String>,
    pub resource: String,

    pub names: Vec<String>,
}

impl BindingContext {
    pub fn names(&self) -> Option<String> {
        Some(self.names.join("\n"))
    }

    pub fn json(&self) -> Option<String> {
        Some(
            kubectl_base_cmd(
                self.namespace.as_ref().map(String::as_str),
                "get",
                &self.resource,
            )
            .arg("--output")
            .arg("json")
            .args(&self.names)
            .capture()
            .ok()?
            .stdout_str(),
        )
    }

    pub fn yaml(&self) -> Option<String> {
        Some(
            kubectl_base_cmd(
                self.namespace.as_ref().map(String::as_str),
                "get",
                &self.resource,
            )
            .arg("--output")
            .arg("yaml")
            .args(&self.names)
            .capture()
            .ok()?
            .stdout_str(),
        )
    }

    pub fn describe(&self) -> Option<String> {
        Some(
            kubectl_base_cmd(
                self.namespace.as_ref().map(String::as_str),
                "describe",
                &self.resource,
            )
            .args(&self.names)
            .capture()
            .ok()?
            .stdout_str(),
        )
    }

    pub fn enter(&self) -> Option<String> {
        let mut ctx: ClipboardContext = ClipboardProvider::new().ok()?;

        ctx.set_contents(self.names()?).ok();

        None
    }
}
