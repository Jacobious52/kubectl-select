use clap::Clap;
use skim::prelude::*;
use std::collections::HashMap;
use std::sync::Mutex;

mod kubectl;
use kubectl::*;

mod bindings;
use bindings::*;

#[derive(Clap)]
#[clap(version = "0.1", author = "Jacobious52")]
struct Opts {
    #[clap(short, long)]
    namespace: Option<String>,

    #[clap(default_value = "pod")]
    resource: String,

    #[clap(skip)]
    bindings: Arc<Mutex<HashMap<String, Arc<dyn Binding + Send + Sync>>>>,
}

impl Opts {
    // adds the key bindings for skim to use as actions
    fn setup_bindings(&mut self) {
        self.add_binding(Names);
        self.add_binding(Json);
        self.add_binding(Yaml);
        self.add_binding(Describe);
        self.add_binding(Edit);
        self.add_binding(Logs);
        self.add_binding(Cordon);
        self.add_binding(Uncordon);
        self.add_binding(Copy);
    }

    fn add_binding<T: Binding + Send + Sync + 'static>(&mut self, b: T) {
        if self.bindings.lock().unwrap().contains_key(&b.key()) {
            panic!("key {} already bound", b.key());
        }
        self.bindings.lock().unwrap().insert(b.key(), Arc::new(b));
    }

    // run the end to end flow with the current options
    fn run(&self) -> Option<String> {
        // everything builds from a kubectl get <resource> list
        // presented in the same format as kubectl would by through skim for fuzzy search
        let kubectl_output = self.kubectl_get()?;

        let prompt = format!("{} ⎈  ", self.resource);

        let options = SkimOptionsBuilder::default()
            .height(Some("30%"))
            .multi(true)
            .reverse(true)
            .prompt(Some(&prompt))
            .preview(Some(""))
            .preview_window(Some("right:20%"))
            .header(Some(&*kubectl_output.header))
            .expect(Some(
                self.bindings
                    .lock()
                    .unwrap()
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(","),
            ))
            .build()
            .unwrap();

        // put all the items in a channel for skim to read from
        let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();
        for item in kubectl_output.items {
            let _ = tx_item.send(Arc::new(item));
        }

        // so that skim could know when to stop waiting for more items.
        // we do this sync since kubectl buffers until everything is fetched anyway
        drop(tx_item);

        // run skim, get the selected items and the key used to terminate skim
        let (selected_items, key) = Skim::run_with(&options, Some(rx_item))
            .map(|out| (out.selected_items, out.accept_key))
            .unwrap_or_else(|| (Vec::new(), None));

        // anything returned will be printed to stdout
        key.map(|k| self.handle_output(&k, &selected_items))
            .flatten()
    }

    // handles any action such as key binding / exit / accept and returns the output of the action
    fn handle_output(&self, key: &str, selected_items: &[Arc<dyn SkimItem>]) -> Option<String> {
        // pre calculate all the names of the selected items since we only really need the name key
        let names: Vec<String> = selected_items
            .iter()
            .map(|i| i.output().into_owned())
            .collect();

        let binding_context = BindingContext {
            namespace: self.namespace.clone(),
            resource: self.resource.clone(),
            names,
        };

        // run our binding if it exists and can run this resource type, otherwise

        let bindings = self.bindings.lock().unwrap();
        let binding = bindings.get(key)?;

        if !binding.runs_for(&self.resource) {
            return Some(format!(
                "{} does not work for resource type {}",
                binding.description(),
                self.resource
            ));
        }
        binding.run(&binding_context)
    }

    // kubectl get with options for the resource specified in the arguments
    // kubectl get -n <namspace>? <resource>
    // todo: add ability to change args based on resource with custom-columns
    // for example: pods might want to always add the node and ip name without full -o
    fn kubectl_get(&self) -> Option<KubectlOutput> {
        let builder = kubectl_base_cmd(self.namespace.as_deref(), "get", self.resource.clone());

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
                .map(|i| KubectlItem::new(i, self.resource.clone(), self.bindings.clone()))
                .collect(),
        };

        Some(out)
    }
}

fn main() {
    let mut opts: Opts = Opts::parse();
    opts.setup_bindings();

    // the user can pipe to a reader of choice if desired
    // so just print to stdout
    // perhaps in future add optional inbuilt readers such as `bat`
    if let Some(final_output) = opts.run() {
        print!("{}", final_output);
    }
}
