# `cargo-crev` tips & tricks

### Find the most heavy dependencies

``` text,ignore
$ cargo crev crate verify --recursive
status reviews     downloads    owner  issues lines  geiger flgs crate                version         latest_t
pass    1  1   932599    973037  1  1   0/0      40       3      fuchsia-cprng        0.1.1           =
pass    1  1  3299093   5112798  1  1   0/0     137       0      version_check        0.1.5           =

(...)

warn    0  0  1668529   2824521 42 20   2/2  414818    5240 CB   tokio-udp            0.1.3
warn    0  0   635827   3488232 44 19   3/3  439497    7088 CB   tokio                0.1.22
warn    0  3   308272   5924679 47 20   3/3  494108    8155 CB   hyper                0.12.33         ↓0.10.2
warn    0  0   849186   2003601 62 26   3/3  550963   17683 CB   hyper-tls            0.3.2
warn    0  0   141213   2088970 86 38   3/3  903083   20094 CB   reqwest              0.9.20
warn    0  1      825      6205 90 40   3/3  916613   20106 CB   crates_io_api        0.5.1           ↓0.3.0
warn    0  1     7417    355870 85 42   0/0  813073   18826 CB   cargo                0.38.0          ↓0.20.0
```

Explanation:

`--recursive` will make the dependency scanner track the stats recursively
(including all the dependencies).

As you can see `cargo` library brings 831 thousands lines of code with it, 18
thousands of which are `unsafe`. This code has 85 total owners on crates.io,
within 42 distinct groups of ownership.

See `cargo crev crate verify --help` more details. This future is still under
active development.

### Find the top contributors

``` text,ignore
> cargo crev crate mvp
 26 FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE https://github.com/dpc/crev-proofs
 19 X98FCpyv5I7z-xv4u-xMWLsFgb_Y0cG7p5xNFHSjbLA https://github.com/kornelski/crev-proofs
 17 6OZqHXqyUAF57grEY7IVMjRljdd9dgDxiNtr1NF1BdY https://github.com/MaulingMonkey/crev-proofs
 12 lr2ldir9XdBsKQkW3YGpRIO2pxhtSucdzf3M5ivfv4A https://git.sr.ht/~icefox/crev-proofs
 11 Qf4cHJBEoho61fd5zoeweyrFCIZ7Pb5X5ggc5iw4B50 https://github.com/kornelski/crev-proofs
  9 ZOm7om6WZyEf3SBmDC69BXs8sc1VPniYx7Nfz2Du6hM https://gitlab.com/KonradBorowski/crev-proofs
  6 VylyTuk8CMGqIxgHixWaqfiUn3xZyzOA1wFrQ0sR1As https://github.com/BurntSushi/crev-proofs
  3 ZGgmIacywCRKLa33k7W04VFcK-glDkcBXKG4oF7t--4 https://github.com/kpcyrd/crev-proofs
  2 ZCBwWlOeJyU79adJqX9-irGH5wrmuYxUPXeSrFKuayg https://github.com/Lokathor/crev-proofs
  1 aD4K0g6AcSKUDp3VPF7u4hM94zEkqjWeRQwmabLBcV0 https://github.com/Mark-Simulacrum/crev-proofs
  1 FBkykBV6YaqAaGoUXyvd-XkEqDYxQNM7EUnZ2nuy-XQ https://github.com/Canop/crev-proofs
  1 pt_he2sLPg2w2u4YN7lj-6Gvu25R8aN6ZCcuQFzxC1g https://gitlab.com/phgsng/crev-proofs
  1 YWfa4SGgcW87fIT88uCkkrsRgIbWiGOOYmBbA1AtnKA https://github.com/oherrala/crev-proofs
```

Explanation:

`cargo crev crate mvp` counts number of dependencies reviewed by each trusted
ID. These are the people you doing most heavy work for you.

You can add `--trust none` argument to discover people that did review some of
your dependencies, yet you still don't have them in your WoT.

### Deal with too many dependencies displayed at once

`cargo crev verify` can be given flags and arguments to narrow down the crate
selection.

``` text,ignore
$ cargo crev crate verify structopt
status reviews     downloads    owner  issues lines  geiger flgs crate                version         latest_t
pass    1  2   883719   6669131  1/1    0/0     141      10      atty                 0.2.13          =
pass    1  1  1400432  17086407  3/3    0/0     875       0 CB   bitflags             1.1.0           =
none    0  0  1593839   5174664  0/1    0/0     900       0      textwrap             0.11.0
none    0  2   256588   1041610  0/1    0/0    2110       0      structopt            0.2.18          ↓0.2.16
none    0  2   269250   1055834  0/1    0/0     930       0      structopt-derive     0.2.18          ↓0.2.16
none    0  0   473513   6201912  3/6    0/0     419       0      unicode-width        0.1.6
none    0  0  3718852   5903147  0/1    0/0     737      23      ansi_term            0.11.0
none    0  0  1030341  13059608  1/1    0/0     904       0      quote                0.6.13
none    0  0  1949014   9339336  2/2    0/0    3541       0 CB   proc-macro2          0.4.30
pass    1  1  1490421   2020940  1/1    0/0     308       0      heck                 0.3.1           =
pass    1  1  6226050  11896441  3/6    0/0     514       0      unicode-xid          0.1.0           =
none    0  0  3532538   5637039  0/3    0/0     989       0      vec_map              0.8.1
pass    1  1  1474739   7299608  0/1    0/0     677       0      strsim               0.8.0           =
pass    1  1  2158613   2226697  1/1    0/0      13       0 CB   winapi-x86_64-pc-windows-gnu 0.4.0           =
none    0  0  2106302   2173081  1/1    0/0      13       0 CB   winapi-i686-pc-windows-gnu 0.4.0
none    0  1  1593422   6323276  1/1    0/0   11198       1      clap                 2.33.0          ↓2.32.0
none    0  0   955234   3870084  3/6    0/0    6062       0      unicode-segmentation 1.3.0
none    0  0   620721  14604329  1/1    0/0   31985      35 CB   syn                  0.15.44
pass    1  1   838269  18472077  4/4    0/0   58231      37 CB   libc                 0.2.62          =
pass    1  2  1177413  10994523  1/1    0/0  160451     197 CB   winapi               0.3.7           ↑0.3.8
```

An optional argument to verify only a given crate and its dependencies.

``` text,ignore
> cargo crev crate verify structopt --skip-indirect
status reviews     downloads    owner  issues lines  geiger flgs crate                version         latest_t
none    0  2   256588   1041610  0/1    0/0    2110       0      structopt            0.2.18          ↓0.2.16
none    0  2   269250   1055834  0/1    0/0     930       0      structopt-derive     0.2.18          ↓0.2.16
none    0  1  1593422   6323276  1/1    0/0   11198       1      clap                 2.33.0          ↓2.32.0
```

`--skip-indirect` displays only a direct dependencies.

Check the `cargo crev crate verify --help` output for more helpful flags.

### Use `cargo crev` to recommend dependencies

``` text,ignore
> cargo crev crate search logging
       3 rand
       2 log
       1 env_logger
       1 slog-term
       1 directories
       1 slog-async
       1 ct-logs
       1 wild
       1 imagequant-sys
       0 unicode-segmentation
```

`cargo crev crate search <query>` will query crates.io for crate maching a given
query, and then sort them from the most trustworthy.

This features is still new and is planed to be expanded and improved.
