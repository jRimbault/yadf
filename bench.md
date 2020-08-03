Other software:
- [fdupes 1.6.1](https://github.com/adrianlopezroche/fdupes)
- [jdupes 1.14.0](https://github.com/jbruchon/jdupes)
- [ddh master](https://github.com/darakian/ddh)
- [fddf 1.7.0](https://github.com/birkenfeld/fddf)
- [rmlint 2.9.0](https://github.com/sahib/rmlint)


Output of [`bench.sh`](bench.sh):

```
Benchmark #1: yadf ~
  Time (mean ± σ):      3.230 s ±  0.325 s    [User: 4.571 s, System: 5.598 s]
  Range (min … max):    2.927 s …  3.629 s    10 runs

Benchmark #2: fdupes -r ~
  Time (mean ± σ):     57.421 s ±  0.132 s    [User: 24.870 s, System: 32.467 s]
  Range (min … max):   57.181 s … 57.597 s    10 runs

Benchmark #3: jdupes -r ~
  Time (mean ± σ):      8.759 s ±  0.022 s    [User: 3.828 s, System: 4.866 s]
  Range (min … max):    8.726 s …  8.782 s    10 runs

Benchmark #4: ddh ~
  Time (mean ± σ):      7.010 s ±  0.089 s    [User: 22.839 s, System: 11.028 s]
  Range (min … max):    6.930 s …  7.220 s    10 runs

Benchmark #5: rmlint --hidden ~
  Time (mean ± σ):     15.478 s ±  0.084 s    [User: 39.156 s, System: 15.248 s]
  Range (min … max):   15.298 s … 15.606 s    10 runs

Benchmark #6: fddf ~
  Time (mean ± σ):      4.959 s ±  0.035 s    [User: 9.736 s, System: 12.407 s]
  Range (min … max):    4.878 s …  5.001 s    10 runs

Summary
  'yadf ~' ran
    1.54 ± 0.16 times faster than 'fddf ~'
    2.17 ± 0.22 times faster than 'ddh ~'
    2.71 ± 0.27 times faster than 'jdupes -r ~'
    4.79 ± 0.48 times faster than 'rmlint --hidden ~'
   17.78 ± 1.79 times faster than 'fdupes -r ~'
```

Results as a csv [table](bench.csv).

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
