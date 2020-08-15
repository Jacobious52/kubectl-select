use clipboard::{ClipboardContext, ClipboardProvider};

use crate::kubectl::kubectl_base_cmd;

// trait for being a key binding action that can be run after skim
// provides the infomation needed to fully describe and action a binding
pub trait Binding {
    // function is called with the context of the selected items
    // return any output wanted sent to stdout
    fn run(&self, ctx: &BindingContext) -> Option<String>;

    // the key binding that this binding corresponds to
    // example: ctrl-k
    fn key(&self) -> String;

    // the human readable name of the action
    fn description(&self) -> String;

    // a list of resource types that this binding works for
    // return an empty list for working on all resource types
    // currently have to provide all short names for resource type
    // example: vec!["pods", "pod", "po"]
    // there are some common ones as a helper on the binding context type
    fn accepts(&self) -> Vec<String>;

    fn runs_for(&self, resource: &str) -> bool {
        let accepts = self.accepts();
        accepts.is_empty() || accepts.iter().any(|r| r == resource)
    }
}

// provides the binding trait implementatins with some context for running
// this includes namespace, resource and a list of select names
pub struct BindingContext {
    pub namespace: Option<String>,
    pub resource: String,

    pub names: Vec<String>,
}

impl BindingContext {
    #[allow(dead_code)]
    fn accepts_pods() -> Vec<String> {
        vec!["pods".into(), "pod".into(), "po".into()]
    }

    #[allow(dead_code)]
    fn accepts_nodes() -> Vec<String> {
        vec!["nodes".into(), "node".into(), "no".into()]
    }

    #[allow(dead_code)]
    fn accepts_service_accounts() -> Vec<String> {
        vec!["serviceaccounts".into(), "sa".into()]
    }
}

// Names returns all the names of the selected items
pub struct Names;

impl Binding for Names {
    fn run(&self, ctx: &BindingContext) -> Option<String> {
        Some(ctx.names.join("\n"))
    }
    fn key(&self) -> String {
        "ctrl-n".into()
    }
    fn description(&self) -> String {
        "Names".into()
    }
    fn accepts(&self) -> Vec<String> {
        Vec::new()
    }
}

// Json returns a json output of the selected items
// kubectl get -o json <resource> <items..>
pub struct Json;

impl Binding for Json {
    fn run(&self, ctx: &BindingContext) -> Option<String> {
        Some(
            kubectl_base_cmd(ctx.namespace.as_deref(), "get", ctx.resource.clone())
                .arg("--output")
                .arg("json")
                .args(&ctx.names)
                .capture()
                .ok()?
                .stdout_str(),
        )
    }
    fn key(&self) -> String {
        "ctrl-j".into()
    }
    fn description(&self) -> String {
        "Json".into()
    }
    fn accepts(&self) -> Vec<String> {
        Vec::new()
    }
}

// Yaml returns a yaml output of the selected items
// kubectl get -o yaml <resource> <items..>
pub struct Yaml;

impl Binding for Yaml {
    fn run(&self, ctx: &BindingContext) -> Option<String> {
        Some(
            kubectl_base_cmd(ctx.namespace.as_deref(), "get", ctx.resource.clone())
                .arg("--output")
                .arg("yaml")
                .args(&ctx.names)
                .capture()
                .ok()?
                .stdout_str(),
        )
    }
    fn key(&self) -> String {
        "ctrl-y".into()
    }
    fn description(&self) -> String {
        "Yaml".into()
    }
    fn accepts(&self) -> Vec<String> {
        Vec::new()
    }
}

// Describe returns a kubectl describe output of the selected items
// kubectl describe <resource> <items..>
pub struct Describe;

impl Binding for Describe {
    fn run(&self, ctx: &BindingContext) -> Option<String> {
        Some(
            kubectl_base_cmd(ctx.namespace.as_deref(), "describe", ctx.resource.clone())
                .args(&ctx.names)
                .capture()
                .ok()?
                .stdout_str(),
        )
    }
    fn key(&self) -> String {
        "ctrl-d".into()
    }
    fn description(&self) -> String {
        "Describe".into()
    }
    fn accepts(&self) -> Vec<String> {
        Vec::new()
    }
}

// Copy copies the selected items to the clipboard in a newline per item format
// param key defines the key used to trigger this, so it can be changed by the user
// by default use "" which is return
#[derive(Default)]
pub struct Copy {
    key: String,
}

impl Binding for Copy {
    fn run(&self, ctx: &BindingContext) -> Option<String> {
        let mut clip_ctx: ClipboardContext = ClipboardProvider::new().ok()?;
        clip_ctx.set_contents(Names.run(ctx)?).ok();
        None
    }
    fn key(&self) -> String {
        self.key.clone()
    }
    fn description(&self) -> String {
        "Copy".into()
    }
    fn accepts(&self) -> Vec<String> {
        Vec::new()
    }
}

// Cordon returns a kubect cordon on a node or nodes
// kubectl cordon node
pub struct Cordon;

impl Binding for Cordon {
    fn run(&self, ctx: &BindingContext) -> Option<String> {
        Some(
            kubectl_base_cmd(ctx.namespace.as_deref(), "cordon", None)
                .args(&ctx.names)
                .capture()
                .ok()?
                .stdout_str(),
        )
    }
    fn key(&self) -> String {
        "ctrl-n".into()
    }
    fn description(&self) -> String {
        "Cordon".into()
    }
    fn accepts(&self) -> Vec<String> {
        BindingContext::accepts_nodes()
    }
}

// Uncordon returns a kubect uncordon on a node or nodes
// kubectl uncordon node
pub struct Uncordon;

impl Binding for Uncordon {
    fn run(&self, ctx: &BindingContext) -> Option<String> {
        Some(
            kubectl_base_cmd(ctx.namespace.as_deref(), "uncordon", None)
                .args(&ctx.names)
                .capture()
                .ok()?
                .stdout_str(),
        )
    }
    fn key(&self) -> String {
        "ctrl-m".into()
    }
    fn description(&self) -> String {
        "Uncordon".into()
    }
    fn accepts(&self) -> Vec<String> {
        BindingContext::accepts_nodes()
    }
}
