# k8s:fwd — config-based kubernetes multi-port forwarding

A tool for handling port-forwards to multiple services and across multiple clusters.

To install the latest version, run:

```shell
cargo install k8sfwd
```

Please note that the application internally relies on `kubectl`, so it needs to be present in your path.
If `kubectl` is not on your path, you may specify it via the `--kubectl` argument or
the `KUBECTL_PATH` environment variable.

Depending on your configuration, you'll be greeted with something along the lines of:

```
██╗░░██╗░█████╗░░██████╗░░░░░███████╗██╗░░░░░░░██╗██████╗
██║░██╔╝██╔══██╗██╔════╝░██╗░██╔════╝██║░░██╗░░██║██╔══██╗
█████═╝░╚█████╔╝╚█████╗░░╚═╝░█████╗░░╚██╗████╗██╔╝██║░░██║
██╔═██╗░██╔══██╗░╚═══██╗░██╗░██╔══╝░░░████╔═████║░██║░░██║
██║░╚██╗╚█████╔╝██████╔╝░╚═╝░██║░░░░░░╚██╔╝░╚██╔╝░██████╔╝
╚═╝░░╚═╝░╚════╝░╚═════╝░░░░░░╚═╝░░░░░░░╚═╝░░░╚═╝░░╚═════╝
k8s:fwd 0.1.0
Using kubectl version v1.24.12-dispatcher
Using config from .k8sfwd

Forwarding to the following targets:
#0 Items API (Staging)
   target:  service/foo.test-api
   context: (default)
   cluster: (default)
#1 Items API (Production)
   target:  pod/foo-59b58f5d68-6t6bh.test-api
   context: (default)
   cluster: production

Spawning child processes:
#0: Error from server (NotFound): pods "foo-59b58f5d68-6t6bh" not found
#0: Process exited with exit status: 1 - will retry in 5 sec
#1: Forwarding from 127.0.0.1:5012 -> 80
#1: Forwarding from 127.0.0.1:46737 -> 8080
#1: Forwarding from [::1]:5012 -> 80
#1: Forwarding from [::1]:46737 -> 8080
#0: Error from server (NotFound): pods "foo-59b58f5d68-6t6bh" not found
#0: Process exited with exit status: 1 - will retry in 5 sec
```

## Configuration

If no configuration file is specified when starting the application, it will recursively look for
a `.k8sfwd` file in the current directory hierarchy. If a file is specified on program launch,
this configuration is used instead.

```yaml
---
version: 0.1.0
config:
  # Optional: Number of seconds to wait before attempting to re-establish
  # a broken connection.
  retry_delay_sec: 5.0
targets:
  - name: Test API (Staging)    # Optional, for display purposes.
    target: foo                 # The name of the resource to forward to.
    type: service               # Can be service, deployment or pod.
    namespace: bar              # The namespace of the resource.
    context: null               # Optional; will default to current context.
    cluster: null               # Optional; will default to current cluster.
    ports:
      - "5012:80"               # Forward resource port 80 to local port 5012.
      - "8080"                  # Forward resource port 8080 to random local port. 
  - name: Test API (Production)
    target: foo-59b58f5d68-6t6bh
    type: pod
    namespace: bar
    cluster: production
    listen_addrs:               # Select the listen addresses; defaults to `localhost`.
      - "127.1.0.1"
    ports:
      - "5012:80"
```
