use crate::bindings::Binding;
use skim::prelude::*;
use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};
use subprocess::Exec;
use tabwriter::TabWriter;

type BindingMap = HashMap<String, Arc<dyn Binding + Sync + Send>>;

// provides the base command for kubectl as a Exec builder to expand on
// kubectl -n <namespace>? <command> <resource>
pub fn kubectl_base_cmd<T: Into<Option<String>>>(
    namespace: Option<&str>,
    command: &str,
    resource: T,
) -> subprocess::Exec {
    let mut builder = Exec::cmd("kubectl").arg(command);
    if let Some(resource) = resource.into() {
        builder = builder.arg(resource);
    }
    if let Some(namespace) = namespace {
        builder = builder.arg("--namespace").arg(namespace);
    }
    builder
}

// encapsulates the result of a kubectl get output list
#[derive(Clone)]
pub struct KubectlOutput {
    pub header: String,
    pub items: Vec<KubectlItem>,
}

// provider an encapsulation over a row in kubectl get
// whitespace separated strings
// first token will usually be name of resource
#[derive(Clone)]
pub struct KubectlItem {
    inner: String,
    resource: String,
    bindings: Arc<Mutex<BindingMap>>,
}

impl KubectlItem {
    pub fn new(inner: String, resource: String, bindings: Arc<Mutex<BindingMap>>) -> Self {
        KubectlItem {
            inner,
            resource,
            bindings,
        }
    }
}

// implement skim trait so we use it in skim and as returned selected items
impl SkimItem for KubectlItem {
    fn display(&self) -> Cow<AnsiString> {
        Cow::Owned(self.inner.as_str().into())
    }

    fn text(&self) -> Cow<str> {
        Cow::Borrowed(&self.inner)
    }

    // show a list of commands for this time
    // probably could be moved into a global main once you know the resource
    // but this allows for per item previews in the future if needed
    fn preview(&self) -> ItemPreview {
        let mut tab_writer = TabWriter::new(vec![]);

        // inject global always available bindings from skim
        // gross way to do it
        let toggle_preview = "\u{1b}[31mToggle Preview\t\u{1b}[33mctrl-p\u{1b}[0m".to_string();
        let always_keys: Vec<String> = vec![toggle_preview];

        // get preview for each binding this resource works for and return a newline per result
        let mut sorted_previews = self
            .bindings
            .lock()
            .unwrap()
            .values()
            .filter(|b| b.runs_for(&self.resource))
            .map(|b| b.preview())
            .chain(always_keys)
            .collect::<Vec<_>>();
        sorted_previews.sort();

        let preview_str = sorted_previews.join("\n");

        tab_writer.write_all(preview_str.as_bytes()).unwrap();
        tab_writer.flush().unwrap();

        let tabbed_str = String::from_utf8(tab_writer.into_inner().unwrap()).unwrap();

        ItemPreview::AnsiText(tabbed_str)
    }

    // output is what's returned from selected items (unless you do some trait downcasting)
    // it returns the name of the resource (first value).
    fn output(&self) -> Cow<str> {
        Cow::Borrowed(&self.inner)
        //Cow::Borrowed(self.inner.split_whitespace().next().unwrap_or(&self.inner))
    }
}
