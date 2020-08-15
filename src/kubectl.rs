use crate::bindings::Binding;
use skim::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use subprocess::Exec;

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

    // for the preview window which we don't use atm
    // could be kubectl describe, but would be slow if not async
    fn preview(&self) -> ItemPreview {
        ItemPreview::AnsiText(
            self.bindings
                .lock()
                .unwrap()
                .values()
                .filter(|b| b.runs_for(&self.resource))
                .map(|b| format!("{} {}", b.description(), b.key()))
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }

    // output is what's returned from selected items (unless you do some trait downcasting)
    // it returns the name of the resource (first value).
    fn output(&self) -> Cow<str> {
        Cow::Borrowed(self.inner.split_whitespace().next().unwrap_or(&self.inner))
    }
}
