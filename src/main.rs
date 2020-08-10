use clap::Clap;
use clipboard::{ClipboardContext, ClipboardProvider};
use skim::prelude::*;
use std::collections::HashMap;
use subprocess::Exec;

#[derive(Debug, Clone)]
struct KubectlItem {
    inner: String,
}

impl KubectlItem {
    fn new(inner: String) -> Self {
        KubectlItem { inner: inner }
    }
}

impl SkimItem for KubectlItem {
    fn display(&self) -> Cow<AnsiString> {
        Cow::Owned(self.inner.as_str().into())
    }

    fn text(&self) -> Cow<str> {
        Cow::Borrowed(&self.inner)
    }

    fn preview(&self) -> ItemPreview {
        ItemPreview::AnsiText(format!("{}", self.inner))
    }

    fn output(&self) -> Cow<str> {
        Cow::Borrowed(self.inner.split_whitespace().next().unwrap_or(&self.inner))
    }
}

#[derive(Debug, Clone)]
struct KubectlOutput {
    header: String,
    items: Vec<KubectlItem>,
}

struct BindingContext {
    namespace: Option<String>,
    resource: String,

    names: Vec<String>,
}

fn kubectl_base_cmd(namespace: Option<&str>, command: &str, resource: &str) -> subprocess::Exec {
    let mut builder = Exec::cmd("kubectl").arg(command).arg(resource);
    if let Some(namespace) = namespace.clone() {
        builder = builder.arg("--namespace").arg(namespace);
    }
    builder
}

impl BindingContext {
    fn names(&self) -> Option<String> {
        Some(self.names.join("\n"))
    }

    fn json(&self) -> Option<String> {
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

    fn yaml(&self) -> Option<String> {
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

    fn describe(&self) -> Option<String> {
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

    fn enter(&self) -> Option<String> {
        let mut ctx: ClipboardContext = ClipboardProvider::new().ok()?;

        ctx.set_contents(self.names()?).ok();

        None
    }
}

#[derive(Clap)]
#[clap(version = "0.1", author = "Jacobious52")]
struct Opts {
    #[clap(short, long)]
    namespace: Option<String>,

    #[clap(default_value = "pod")]
    resource: String,

    #[clap(skip)]
    key_bindings: HashMap<String, fn(&BindingContext) -> Option<String>>,
}

impl Opts {
    fn setup_bindings(&mut self) {
        self.key_bindings.insert("".into(), BindingContext::enter);
        self.key_bindings
            .insert("ctrl-y".into(), BindingContext::yaml);
        self.key_bindings
            .insert("ctrl-j".into(), BindingContext::json);
        self.key_bindings
            .insert("ctrl-n".into(), BindingContext::names);
        self.key_bindings
            .insert("ctrl-d".into(), BindingContext::describe);
    }

    fn run(&self) -> Option<String> {
        let kubectl_output = self.kubectl_get()?;

        let options = SkimOptionsBuilder::default()
            .height(Some("30%"))
            .multi(true)
            .reverse(true)
            .preview(None)
            .header(Some(&*kubectl_output.header))
            .expect(Some(
                self.key_bindings
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(","),
            ))
            .build()
            .unwrap();

        let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();
        for item in kubectl_output.items {
            let _ = tx_item.send(Arc::new(item));
        }

        // so that skim could know when to stop waiting for more items.
        drop(tx_item);

        let (selected_items, key) = Skim::run_with(&options, Some(rx_item))
            .map(|out| (out.selected_items, out.accept_key))
            .unwrap_or_else(|| (Vec::new(), None));

        return key
            .map(|k| self.handle_output(&k, &selected_items))
            .flatten();
    }

    fn handle_output(&self, key: &str, selected_items: &[Arc<dyn SkimItem>]) -> Option<String> {
        let names: Vec<String> = selected_items
            .iter()
            .map(|i| i.output().into_owned())
            .collect();

        let binding_context = BindingContext {
            namespace: self.namespace.clone(),
            resource: self.resource.clone(),
            names,
        };

        self.key_bindings.get(key)?(&binding_context)
    }

    fn kubectl_get(&self) -> Option<KubectlOutput> {
        let builder = kubectl_base_cmd(
            self.namespace.as_ref().map(String::as_str),
            "get",
            &self.resource,
        );

        let lines: Vec<String> = builder
            .capture()
            .ok()?
            .stdout_str()
            .lines()
            .map(String::from)
            .collect();

        let out = KubectlOutput {
            header: lines.first()?.into(),
            items: lines
                .iter()
                .skip(1)
                .cloned()
                .map(KubectlItem::new)
                .collect(),
        };

        Some(out)
    }
}

fn main() {
    let mut opts: Opts = Opts::parse();
    opts.setup_bindings();

    if let Some(final_output) = opts.run() {
        println!("{}", final_output);
    }
}
