Other software:
- [fdupes 1.6.1](https://github.com/adrianlopezroche/fdupes)
- [jdupes 1.14.0](https://github.com/jbruchon/jdupes)
- [ddh 0.11.2](https://github.com/darakian/ddh)
- [fddf 1.7.0](https://github.com/birkenfeld/fddf)
- [rmlint 2.9.0](https://github.com/sahib/rmlint)
- [dupe-krill 1.4.4](https://github.com/kornelski/dupe-krill)


Output of [`bench.sh`](bench.sh):

```
Benchmark #1: fdupes -r ~
  Time (mean ± σ):     54.487 s ±  0.323 s    [User: 23.467 s, System: 30.967 s]
  Range (min … max):   53.657 s … 54.740 s    10 runs

Benchmark #2: jdupes -r ~
  Time (mean ± σ):      8.538 s ±  0.250 s    [User: 3.780 s, System: 4.703 s]
  Range (min … max):    8.350 s …  9.001 s    10 runs

Benchmark #3: rmlint --hidden ~
  Time (mean ± σ):     14.786 s ±  0.064 s    [User: 38.037 s, System: 14.466 s]
  Range (min … max):   14.679 s … 14.896 s    10 runs

Benchmark #4: ddh ~
  Time (mean ± σ):      6.320 s ±  0.121 s    [User: 18.878 s, System: 9.846 s]
  Range (min … max):    6.174 s …  6.531 s    10 runs

Benchmark #5: dupe-krill -s -d ~
  Time (mean ± σ):      7.570 s ±  0.058 s    [User: 4.721 s, System: 2.814 s]
  Range (min … max):    7.517 s …  7.684 s    10 runs

Benchmark #6: fddf ~
  Time (mean ± σ):      4.835 s ±  0.060 s    [User: 9.519 s, System: 12.036 s]
  Range (min … max):    4.744 s …  4.925 s    10 runs

Benchmark #7: yadf ~
  Time (mean ± σ):      3.511 s ±  0.011 s    [User: 5.185 s, System: 6.032 s]
  Range (min … max):    3.496 s …  3.528 s    10 runs

Summary
  'yadf ~' ran
    1.38 ± 0.02 times faster than 'fddf ~'
    1.80 ± 0.03 times faster than 'ddh ~'
    2.16 ± 0.02 times faster than 'dupe-krill -s -d ~'
    2.43 ± 0.07 times faster than 'jdupes -r ~'
    4.21 ± 0.02 times faster than 'rmlint --hidden ~'
   15.52 ± 0.10 times faster than 'fdupes -r ~'
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
