# kubeshim

*Warning:* This code isn't great and may try to eat your cat.

## What is it?

Wraps kubernetes tools and provides a tiny HTTP CONNECT to SOCKS5 proxy
so you can use a bastion host without as much trouble.

## Install

Latest builds can be found in the artifacts [here][release]

Build for yourself with [rust][rust]

The config file has documentation about what each section is for

```shell
cp kubeshim.yaml.example $HOME/.config/kubeshim.yaml && vim $HOME/.config/kubeshim.yaml
export KUBESHIM_ROOT="$HOME/.kubeshim/" # Add this to your startup
mkdir -p $KUBESHIM_ROOT/shims
export PATH="$HOME/.kubeshim/shims:$PATH" # Also add this to your startup
cp exec_script $KUBESHIM_ROOT/
for name in "apps" "that" "you" "want"; do
    ln -s $KUBESHIM_ROOT/exec_script $KUBESHIM_ROOT/shims/$name
done
```

## Usage

1. Make sure you're on the kubectl context that you want to use and you have
a mathcing entry in kubeshim.yaml
1. Make sure that you have your socks proxy open

### Exec script

1. Follow install steps above
1. Run command like normal

### Manual

`kubeshim run -- [command] <args>...`

## Debugging

`kubeshim -v run -- [command] <args>...`

Use more `v`'s to get more details.

If you set `KUBESHIM_DEBUG=1` (or anything really) the exec script will set up
a bunch of debugging.

[release]: https://github.com/xaocon/kubeshim/actions/workflows/create.yaml
[rust]: https://rustup.rs/
