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
  Time (mean ± σ):     54.979 s ±  1.321 s    [User: 23.637 s, System: 31.288 s]
  Range (min … max):   53.823 s … 58.642 s    10 runs

Benchmark #2: jdupes -r ~
  Time (mean ± σ):      8.441 s ±  0.030 s    [User: 3.766 s, System: 4.619 s]
  Range (min … max):    8.386 s …  8.482 s    10 runs

Benchmark #3: rmlint --hidden ~
  Time (mean ± σ):     13.618 s ±  0.046 s    [User: 36.334 s, System: 13.979 s]
  Range (min … max):   13.560 s … 13.686 s    10 runs

Benchmark #4: ddh ~
  Time (mean ± σ):      7.739 s ±  0.031 s    [User: 31.070 s, System: 24.549 s]
  Range (min … max):    7.692 s …  7.788 s    10 runs

Benchmark #5: dupe-krill -s -d ~
  Time (mean ± σ):      7.561 s ±  0.052 s    [User: 4.761 s, System: 2.765 s]
  Range (min … max):    7.497 s …  7.674 s    10 runs

Benchmark #6: fddf ~
  Time (mean ± σ):      4.836 s ±  0.057 s    [User: 9.468 s, System: 11.975 s]
  Range (min … max):    4.731 s …  4.930 s    10 runs

Benchmark #7: yadf ~
  Time (mean ± σ):      3.538 s ±  0.081 s    [User: 5.337 s, System: 6.247 s]
  Range (min … max):    3.312 s …  3.589 s    10 runs

Summary
  'yadf ~' ran
    1.37 ± 0.04 times faster than 'fddf ~'
    2.14 ± 0.05 times faster than 'dupe-krill -s -d ~'
    2.19 ± 0.05 times faster than 'ddh ~'
    2.39 ± 0.06 times faster than 'jdupes -r ~'
    3.85 ± 0.09 times faster than 'rmlint --hidden ~'
   15.54 ± 0.51 times faster than 'fdupes -r ~'
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
