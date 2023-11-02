# Benchmarks

Benchmarks were performed on an [AMD Ryzen 7 4800HS CPU](https://en.wikichip.org/wiki/amd/ryzen_9/3900).

```sh
$ cargo bench --bench <name>
$ critcmp new | tail +3 | sort | sed 's#        ? ?/sec##'
```

## timing
```
quanta/quanta_instant_now       1.00      9.7±0.01ns
quanta/quanta_instant_recent    1.00      1.5±0.10ns
quanta/quanta_now               1.00      9.1±0.65ns
quanta/quanta_now_delta         1.00     18.4±0.04ns
quanta/quanta_raw               1.00      8.9±0.00ns
quanta/quanta_raw_delta         1.00     18.2±0.03ns
quanta/quanta_raw_scaled        1.00      9.0±0.07ns
quanta/quanta_recent            1.00      1.7±0.00ns
stdlib/instant_delta            1.00      2.2±0.09µs
stdlib/instant_now              1.00  1110.8±48.42ns
```

## contention
```
quanta/now/10                   1.00     25.4±9.78ns
quanta/now/1                    1.00     10.6±3.50ns
quanta/now/11                   1.00     25.5±9.57ns
quanta/now/12                   1.00     22.7±4.48ns
quanta/now/2                    1.00     16.0±6.11ns
quanta/now/3                    1.00     17.4±6.34ns
quanta/now/4                    1.00     17.6±5.65ns
quanta/now/5                    1.00     16.2±4.96ns
quanta/now/6                    1.00     17.9±7.19ns
quanta/now/7                    1.00     16.5±5.65ns
quanta/now/8                    1.00     17.7±6.06ns
quanta/now/9                    1.00     24.4±9.00ns
stdlib/now/10                   1.00  1399.3±84.62ns
stdlib/now/1                    1.00  1187.1±138.65ns
stdlib/now/11                   1.00  1388.3±64.43ns
stdlib/now/12                   1.00  1395.2±60.53ns
stdlib/now/2                    1.00  1433.9±158.59ns
stdlib/now/3                    1.00  1384.1±71.93ns
stdlib/now/4                    1.00  1407.1±159.88ns
stdlib/now/5                    1.00  1367.0±62.41ns
stdlib/now/6                    1.00  1411.6±167.61ns
stdlib/now/7                    1.00  1396.0±83.37ns
stdlib/now/8                    1.00  1390.6±81.05ns
stdlib/now/9                    1.00  1436.7±113.11ns
```
