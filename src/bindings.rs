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

    fn preview(&self) -> String {
        let key_repr = if self.key() == "" {
            "enter".into()
        } else {
            self.key()
        };
        format!(
            "\x1b[31m{}\t\x1b[33m{}\x1b[0m",
            self.description(),
            key_repr
        )
    }
}

// provides the binding trait implementations with some context for running
// this includes namespace, resource and a list of select names, plus vec of columns for each item
pub struct BindingContext {
    pub namespace: Option<String>,
    pub resource: String,

    pub names: Vec<String>,
    pub columns: Vec<Vec<String>>,
}

impl BindingContext {
    #[allow(dead_code)]
    pub fn accepts_pods() -> Vec<String> {
        vec!["pods".into(), "pod".into(), "po".into()]
    }

    #[allow(dead_code)]
    pub fn accepts_nodes() -> Vec<String> {
        vec!["nodes".into(), "node".into(), "no".into()]
    }

    #[allow(dead_code)]
    pub fn accepts_service_accounts() -> Vec<String> {
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
        "".into()
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

// Edit returns a kubectl edit output of the selected items
// kubectl edit <resource> <items..>
pub struct Edit;

impl Binding for Edit {
    fn run(&self, ctx: &BindingContext) -> Option<String> {
        let tty = std::fs::File::open("/dev/stdout").ok()?;
        Some(
            kubectl_base_cmd(ctx.namespace.as_deref(), "edit", ctx.resource.clone())
                .args(&ctx.names)
                .stdout(subprocess::Redirection::File(tty))
                .capture()
                .ok()?
                .stdout_str(),
        )
    }
    fn key(&self) -> String {
        "ctrl-e".into()
    }
    fn description(&self) -> String {
        "Edit".into()
    }
    fn accepts(&self) -> Vec<String> {
        Vec::new()
    }
}

// Edit returns a kubectl logs output of the selected pod
// kubectl logs <pod..>
pub struct Logs;

impl Binding for Logs {
    fn run(&self, ctx: &BindingContext) -> Option<String> {
        if ctx.names.len() > 1 {
            return Some("Cannot get logs of more than one pod at a time".into());
        }

        // TODO: install signal handler to gracefully exit
        let comms = kubectl_base_cmd(ctx.namespace.as_deref(), "logs", None)
            .arg("--follow")
            .arg("--all-containers")
            .args(&ctx.names)
            .communicate()
            .ok()?;

        let mut comms = comms.limit_size(1024);

        while let Ok((Some(stdout), None)) = comms.read_string() {
            print!("{}", stdout);
        }

        None
    }
    fn key(&self) -> String {
        "ctrl-l".into()
    }
    fn description(&self) -> String {
        "Logs".into()
    }
    fn accepts(&self) -> Vec<String> {
        BindingContext::accepts_pods()
    }
}

// Copy copies the selected items to the clipboard in a newline per item format
pub struct Copy;

impl Binding for Copy {
    fn run(&self, ctx: &BindingContext) -> Option<String> {
        let mut clip_ctx: ClipboardContext = ClipboardProvider::new().ok()?;
        clip_ctx.set_contents(Names.run(ctx)?).ok();
        None
    }
    fn key(&self) -> String {
        "ctrl-space".into()
    }
    fn description(&self) -> String {
        "Copy".into()
    }
    fn accepts(&self) -> Vec<String> {
        Vec::new()
    }
}

// Cordon returns a kubectl cordon on a node or nodes
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
        "ctrl-k".into()
    }
    fn description(&self) -> String {
        "Cordon".into()
    }
    fn accepts(&self) -> Vec<String> {
        BindingContext::accepts_nodes()
    }
}

// Uncordon returns a kubectl uncordon on a node or nodes
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
        "ctrl-u".into()
    }
    fn description(&self) -> String {
        "Uncordon".into()
    }
    fn accepts(&self) -> Vec<String> {
        BindingContext::accepts_nodes()
    }
}

// Column returns the columns of the selected item indexed by the index param
pub struct Column {
    name: String,
    index: usize,
}

impl Column {
    pub fn new(name: String, index: usize) -> Self {
        Column { name, index }
    }
}

impl Binding for Column {
    fn run(&self, ctx: &BindingContext) -> Option<String> {
        Some(
            ctx.columns
                .iter()
                .filter_map(|c| c.get(self.index))
                .map(String::from)
                .collect::<Vec<String>>()
                .join("\n"),
        )
    }
    fn key(&self) -> String {
        format!("f{}", self.index)
    }
    fn description(&self) -> String {
        format!("{}:{}", self.index, self.name)
    }
    fn accepts(&self) -> Vec<String> {
        Vec::new()
    }
}
