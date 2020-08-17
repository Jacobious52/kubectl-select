# kubectl-select

![Rust](https://github.com/Jacobious52/kubectl-select/workflows/Rust/badge.svg)

Rustification of bash script wrapping kubectl in fuzzy search with key binding actions

## What does it do

In short, the tool acts like a kubectl plugin that feeds `kubectl get <resource>` into a fuzzy finding interface. It also sets up key bindings for some common actions.
It aims to speed up repetive `kubectl get` and search for resource name before performing actions.

## Demo

Coming soon!

## Install

Not published to crates.io yet, so can build from source

`cargo install kubectl-select --git "https://github.com/Jacobious52/kubectl-select.git"`

> On linux you need to install `libxcb-composite0-dev` first. `sudo apt install libxcb-composite0-dev`

## Why rewriting it in Rust

Mainly to learn more Rust, but also to be more maintainable and less error prone than the bash script and supporting working mulit-select in more places

## How to use

`kubectl select <resource-type> <optional query>`

Will open a fuzzy search interface to further down filter items. Key bindings are displayed for an item as a preview and can perform certain actions.
Tab allows selecting mulitple resources at once.

For example with pods:

- `kubectl select pods` + `ctrl-space` will copy all the names of selected resources to the system clipboard
- `kubectl select pods` + `enter` will print the names to stdout
- `kubectl select pods -w` + `ctrl-d` will print out the `describe` command of each selected item with -w for --output wide

Or nodes:
- `kubectl select nodes` + `ctrl-k` or `ctrl-u` will Cordon or Uncordon selected nodes
- `kubectl select nodes` + `ctrl-y` or `ctrl-j` will print out the yaml or json of the selected nodes

Can export columns with mapped to function keys:
- `kubectl select po` + `f2` to export the 2nd info column which is the status of the pods

Help:
```
kubectl-select 0.1
Jacobious52

USAGE:
    kubectl-select [FLAGS] [OPTIONS] [ARGS]

ARGS:
    <resource>     [default: pod]
    <query>...    

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -w, --wide       

OPTIONS:
    -n, --namespace <namespace>  
```

## Similar Projects

- [kubectl-fzf](https://github.com/bonnefoa/kubectl-fzf)
- [k9s](https://github.com/derailed/k9s)

## Contrubitions

Contrubutions are more than welcome through a PR. 
If it's a larger change please conider raising an issue first for discussion

