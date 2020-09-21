Other software:
- [fdupes 1.6.1](https://github.com/adrianlopezroche/fdupes)
- [jdupes 1.14.0](https://github.com/jbruchon/jdupes)
- [ddh 0.11.3](https://github.com/darakian/ddh)
- [fddf 1.7.0](https://github.com/birkenfeld/fddf)
- [rmlint 2.9.0](https://github.com/sahib/rmlint)
- [dupe-krill 1.4.4](https://github.com/kornelski/dupe-krill)


Output of [`bench.sh`](bench.sh):

```
Benchmark #1: fdupes -r ~
  Time (mean ± σ):     62.750 s ±  0.340 s    [User: 28.247 s, System: 34.419 s]
  Range (min … max):   62.059 s … 63.133 s    10 runs

Benchmark #2: jdupes -z -r ~
  Time (mean ± σ):     11.151 s ±  0.032 s    [User: 5.675 s, System: 5.406 s]
  Range (min … max):   11.106 s … 11.201 s    10 runs

Benchmark #3: rmlint --hidden ~
  Time (mean ± σ):     16.370 s ±  0.040 s    [User: 44.962 s, System: 15.873 s]
  Range (min … max):   16.294 s … 16.407 s    10 runs

Benchmark #4: ddh ~
  Time (mean ± σ):     10.080 s ±  0.125 s    [User: 36.864 s, System: 28.908 s]
  Range (min … max):    9.869 s … 10.211 s    10 runs

Benchmark #5: dupe-krill -s -d ~
  Time (mean ± σ):      8.820 s ±  0.087 s    [User: 5.618 s, System: 3.158 s]
  Range (min … max):    8.712 s …  8.961 s    10 runs

Benchmark #6: fddf -m 0 ~
  Time (mean ± σ):      5.309 s ±  0.056 s    [User: 10.526 s, System: 13.561 s]
  Range (min … max):    5.216 s …  5.373 s    10 runs

Benchmark #7: yadf ~
  Time (mean ± σ):      2.944 s ±  0.015 s    [User: 9.975 s, System: 12.670 s]
  Range (min … max):    2.930 s …  2.978 s    10 runs

Summary
  'yadf ~' ran
    1.80 ± 0.02 times faster than 'fddf -m 0 ~'
    3.00 ± 0.03 times faster than 'dupe-krill -s -d ~'
    3.42 ± 0.05 times faster than 'ddh ~'
    3.79 ± 0.02 times faster than 'jdupes -z -r ~'
    5.56 ± 0.03 times faster than 'rmlint --hidden ~'
   21.32 ± 0.16 times faster than 'fdupes -r ~'
```

Hardware information (extract from `neofetch` and `hwinfo --disk`):

```
OS: Ubuntu 20.04.1 LTS x86_64
Host: XPS 15 9570
Kernel: 5.4.0-42-generic
CPU: Intel i9-8950HK (12) @ 4.800GHz
Memory: 4217MiB / 31755MiB
Disk:
  model: "SK hynix Disk"
  driver: "nvme"
```
