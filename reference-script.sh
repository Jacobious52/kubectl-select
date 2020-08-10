#!/bin/bash

# Author: Jacobious52

if [[ $1 = "--help" ]]; then
    echo "usage: kubectl-view <resource>? <query>?
            Pipes kubectl commands into fzf (brew install fzf) with key bindings.. pretty much kube dashboard for terminal
            Place this script somewhere in your PATH. kubectl plugin list

            Omit <resource> to start in interact 'dashboard' mode:
            # This will open in full screen and begin at api-resource instead
            # All output commands will be piped to `less` instead of `cat`
            # Apon action finish, will return to search instead of exiting
                kubectl view

            Start an empty search for <resource>:
                kubectl view pods
                kubectl view nodes
                kubectl view cm

            Start a prefilled search for <resource> with <query>:
                kubectl view pods coredns
                kubectl view pods pending
                kubectl view nodes shared

            Accepts stdin for fzf search:
                kubectl get pods -n monitoring -l app=prometheus | kubectl view pods

            Key Bindings:
                ctrl-d: kubectl describe
                ctrl-l: kubectl logs
                ctrl-e: kubectl edit
                ctrl-s: kubectl exec -it -- sh (if view nodes will deploy a privileged pod to the node)
                ctrl-r: kubectl exec -it -- <command> # will prompt to enter a command
                ctrl-x: kubectl delete
                ctrl-y: output yaml
                ctrl-j: output json
                ctrl-k: object info
                ctrl-n: copy node name or uncordon node
                ctrl-c: exit
                ctrl-t: drop a toolbox pod on the node. delete when finsihed
                return: pbcopy name

            fzf search syntax https://github.com/junegunn/fzf#search-syntax
    "
    exit 0
fi

# if we supply a resource to view directly, continue to use inline mode. Otherwise give api-resources list and recursively start the script with that resource
resource=$1
if [[ -z "$resource" ]]; then
    kubectl api-resources --verbs=get --no-headers | awk '{{ print $1 }}' | sort -u | fzf --prompt="resource ⎈  " --cycle --exit-0 --layout reverse --bind 'enter:execute#kubectl view {} < /dev/tty > /dev/tty 2>&1#'
    exit 0
fi

# optional query to pre input to the search
shift
query=$@

# check to see if we're running in inline mode (from zsh/bash) or spawned in dashboard mode with the above call
parent=$(ps $(echo $PPID) -o command | tail -1)
[[ "$parent" =~ "fzf" ]] || [[ "$parent" =~ "kubectl-view" ]] && outmode=less || outmode=cat
[[ "$parent" =~ "fzf" ]] || [[ "$parent" =~ "kubectl-view" ]] && display="" || display="--height=30%"

# if we have stdin piped use that for the search input, otherwise make a wide get on the resources
[[ -p /dev/stdin ]] && in=$(</dev/stdin) || in=$(kubectl get $resource -o wide)

[[ "$resource" =~ "po" ]] && cols="1,3,5,6,7" || cols=".."

# the main fzf command with all the args and keybindings
out=$(echo "$in" | fzf --query="$query" --prompt="$resource ⎈  " --multi --bind=ctrl-a:toggle-all --nth=.. --with-nth="$cols" --header-lines=1 --expect=ctrl-n,ctrl-d,ctrl-l,ctrl-s,ctrl-e,ctrl-y,ctrl-j,ctrl-k,ctrl-r --cycle $display --layout reverse)
# the first line is the key pressed
key=$(head -1 <<< "$out")
# get the name of the resource(s) selected
name=$(awk 'NR>1 {print $1};' <<< "$out")

# our privileged podspec for dropping a temporary shell with a image that has some useful tools
temppod='
apiVersion: v1
kind: Pod
metadata:
  name: PODNAME
  namespace: kube-system
  labels:
    app: temp
spec:
  dnsPolicy: ClusterFirst
  nodeName: NODENAME
  hostNetwork: HOSTNETWORKING
  tolerations:
  - operator: "Exists"
  restartPolicy: Never  
  hostPID: true
  volumes:
  - name: host
    hostPath:
      path: /
  containers:
    - name: temp
      image: ubuntu
      securityContext:
        privileged: true
      volumeMounts:
      - name: host
        mountPath: /host
      command:
      - /bin/bash
      - -c
      - "trap : TERM INT; sleep infinity & wait"
'

# match on the key pressed to do different actions
if [[ -n "$name" ]]; then    
    case "$key" in
            ctrl-d)
                # describe
                kubectl describe $resource $name | $outmode
                ;;
            ctrl-e)
                # edit inline
                kubectl edit $resource "$name"
                ;;
            ctrl-l)
                # view logs and give list of containers to select from if more than 1
                if [[ $resource =~ "po" ]]; then 
                    container=$(kubectl get po "$name" -o jsonpath='{.spec.containers[*].name}' | tr " " "\n" | fzf -1 --height 30% --layout reverse)
                    kubectl logs -f "$name" -c "$container" | $outmode
                fi
                ;;
            ctrl-s)
                # if pod, exec sh into a selected container
                if [[ $resource =~ "po" ]]; then
                    container=$(kubectl get po "$name" -o jsonpath='{.spec.containers[*].name}' | tr " " "\n" | fzf -1 --height 30% --layout reverse)
                    kubectl exec -it "$name" -c "$container" sh
                fi
                # if node, drop the temp pod spec
                if [[ $resource =~ "no" ]]; then
                    echo "HostNetworking?"
                    host=$(echo -e "true\nfalse" | fzf --cycle --height=30% --layout reverse)

                    rname="temp-$(shuf -n1 /usr/share/dict/words | tr "[A-Z]" "[a-z]")"

                    podspec=$(echo "${temppod/NODENAME/$name}")
                    podspec=$(echo "${podspec/HOSTNETWORKING/$host}")
                    podspec=$(echo "${podspec/PODNAME/$rname}")

                    echo "$podspec" | kubectl apply -f -

                    until [[ $(kubectl -n kube-system get pod $rname | tail -1 | awk '{ print $3 }') == "Running" ]]
                    do
                        echo "waiting for container '$rname' to start"
                        sleep .5
                    done
                    kubectl -n kube-system exec -it $rname -c "$container" bash
                    kubectl -n kube-system delete pod $rname
                fi
                ;;
            ctrl-r)
                # same as above, but input a custom command to run
                if [[ $resource =~ "po" ]]; then
                    container=$(kubectl get po "$name" -o jsonpath='{.spec.containers[*].name}' | tr " " "\n" | fzf -1 --height 30% --layout reverse)
                    read -p 'command to run: ' cmd
                    kubectl exec -it "$name" -c "$container" $cmd
                fi
                ;;
            ctrl-n)
                # if pod, return the node name of the pod to the clipboard
                if [[ $resource =~ "po" ]]; then
                    kubectl get pod $name -o json | jq -r '.spec.nodeName' | tr -d '\n' | pbcopy
                fi
                # if node, uncordon the node
                # TODO: make cordoned if uncordoned
                if [[ $resource =~ "no" ]]; then
                    kubectl uncordon $name
                fi
                ;;
            ctrl-x)
                # delete the resource
                if [[ $resource =~ "no" ]]; then
                    echo "don't delete nodes you idiot"
                else
                    kubectl delete $resource "$name"
                fi
                ;;
            ctrl-y)
                # print the yaml
                kubectl get $resource "$name" -o yaml | $outmode
                ;;
            ctrl-j)
                # print the json
                kubectl get $resource "$name" -o json | $outmode
                ;;
            ctrl-k)
                # get hand crafted info based on the resource. otherwise describe
                # TODO: make this nicer and do more
                case "$resource" in
                    *"po"*)
                        kubectl get $resource $name -o json | jq '{name: .metadata.name, namespace: .metadata.namespace, labels: .metadata.labels, containers: [.spec.containers[].name]}' | $outmode
                        ;;
                    *"no"*)
                        #TODO: node info with pods
                        kubectl get $resource $name -o json | jq '.metadata.name' | $outmode
                        ;;
                    *)
                        kubectl describe $resource $name | $outmode
                        ;;
                esac
                ;;
            *)
                # if enter pressed just copy name to clipboard
                echo -n "$name" | pbcopy
                ;;
    esac
    
    # finally, if in dashboard mode restart the command with the previous input/resource and selected object
    [[ "$parent" =~ "fzf" ]] || [[ "$parent" =~ "kubectl-view" ]] && echo "$in" | kubectl view $resource $name
fi

