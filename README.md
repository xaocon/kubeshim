# kubeshim

*Warning:* This code isn't great and may try to eat your cat.

## What is it?
Wraps kubernetes tools and provides a tiny HTTP CONNECT to SOCKS5 proxy
so you can use a bastion host without as much trouble.

## Install
Latest builds can be found [here][release]

Build for yourself with [rust][rust]

```shell
$ export KUBESHIM_ROOT="$HOME/.kubeshim/" # Add this to your startup
$ mkdir -p $KUBESHIM_ROOT/shims
$ export PATH="$HOME/.kubeshim/shims:$PATH" # Also add this to your startup
$ cp exec_script $KUBESHIM_ROOT/
$ for name in "apps" "that" "you" "want"; do
    ln -s $KUBESHIM_ROOT/exec_script $KUBESHIM_ROOT/shims/$name
  done
```

[release]: https://github.com/xaocon/kubeshim/actions/workflows/create.yaml
[rust]: https://rustup.rs/
