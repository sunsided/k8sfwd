# k8s:fwd — config-based kubernetes multi-port forwarding

A tool for handling port-forwards to multiple services and across multiple clusters.

For Linux (GNU and musl) you can download pre-built binaries from the [Releases](https://github.com/sunsided/k8sfwd/releases)
page. For other platforms the setup is currently based on [cargo] until platform-specific binaries can be provided.
To manually build and install the latest version for your system (or update to it), run:

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
k8s:fwd 0.3.0 - a Kubernetes multi-cluster port forwarder
Using kubectl version v1.24.12-dispatcher
Using config from 2 locations

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

## Command-Line Options

### Filters

Targets can be selected through prefix filters specified on the command-line. Only
targets (and target names) starting with the specified prefixes will be forwarded.
In the following example, services starting with `foo` and `bar` will be selected:

```shell
k8sfwd foo bar
```

Filters can operate in combination with tags as well:

```shell
k8sfwd -t test foo bar
```

### Tags

Targets can be labeled with tags. When `k8sfwd` is started with one or more space-separated
`--tags` parameters, targets are filtered down to match the selection. If multiple values
are specified (e.g. `--tags foo bar`), any matching tag results in the target being selected.
If two tags are combined with a plus sign (e.g. `--tags foo+bar`) only targets matching both
tags are selected.

| Target tags             | `--tags` argument      | Selected |
|-------------------------|------------------------|----------|
| (none)                  | (none)                 | ✅ yes    |
| (none)                  | `--tags some`          | ❌ no     |
| `["foo", "bar", "baz"]` | (none)                 | ✅ yes    |
| `["foo", "bar", "baz"]` | `--tags fubar`         | ❌ no     |
| `["foo", "bar", "baz"]` | `--tags foo bar`       | ✅ yes    |
| `["foo", "bar", "baz"]` | `--tags bar`           | ✅ yes    |
| `["foo", "bar", "baz"]` | `--tags foo+baz`       | ✅ yes    |
| `["foo", "bar", "baz"]` | `--tags foo+fubar`     | ❌ no     |
| `["foo", "bar", "baz"]` | `--tags foo+baz fubar` | ✅ yes    |
| `["fubar"]`             | `--tags foo+baz fubar` | ✅ yes    |
   

## Configuration

The configuration is provided as a YAML file. 

- If one or more files are specified on program launch via the `--file` argument(s), their configuration is loaded.
- If no configuration file is specified, `k8sfwd` will recursively look for a `.k8sfwd` file in 
  - the current directory hierarchy, 
  - your home directory and 
  - your configuration directory, in that order.

Non-target configuration (e.g., retry delays) are always loaded from the hierarchy stated above regardless
of whether a `--file` argument is present. However,  all target configuration that is not directly specified
through a file pointed to by the `--file` argument is ignored.

See [`k8sfwd-example.yaml`](k8sfwd-example.yaml) for an example.

```yaml
---
version: 0.2.0
config:
  # Optional: Number of seconds to wait before attempting to re-establish
  # a broken connection.
  retry_delay_sec: 5.0
targets:
  - name: Test API (Staging)    # Optional, for display purposes.
    target: foo                 # The name of the resource to forward to.
    tags:                       # Optional, for use with `--tags <tag1> <tag2>+<tag3>`
      - integration
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

## FlatHub

You can run the application from FlatPak using

```shell
flatpak run com.github.sunsided.k8sfwd
```

When installed through FlatHub, the application may not find the `kubectl` binary
depending on how it was installed on your system. In this case, ensure to export the `KUBECTL_PATH`
environment variable, pointing it to the correct path:

```shell
export KUBECTL_PATH=...
```

After this, you may run into issues with `gke-gcloud-auth-plugin`.
See [kubectl auth changes in GKE](https://cloud.google.com/blog/products/containers-kubernetes/kubectl-auth-changes-in-gke?hl=en)
for more information.

[cargo]: https://crates.io/
