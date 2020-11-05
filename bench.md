Other software:

- [fclones 0.8.0](https://github.com/pkolaczk/fclones)
- [jdupes 1.14.0](https://github.com/jbruchon/jdupes)
- [ddh 0.11.3](https://github.com/darakian/ddh)
- [fddf 1.7.0](https://github.com/birkenfeld/fddf)
- [rmlint 2.9.0](https://github.com/sahib/rmlint)
- [dupe-krill 1.4.4](https://github.com/kornelski/dupe-krill)

Output of [`bench.sh`](bench.sh):

```
Benchmark #1: fclones --min-size 0 -R ~
  Time (mean ± σ):      3.686 s ±  0.030 s    [User: 15.705 s, System: 12.778 s]
  Range (min … max):    3.635 s …  3.737 s    10 runs

Benchmark #2: jdupes -z -r ~
  Time (mean ± σ):     11.259 s ±  0.030 s    [User: 5.924 s, System: 5.268 s]
  Range (min … max):   11.224 s … 11.322 s    10 runs

Benchmark #3: rmlint --hidden ~
  Time (mean ± σ):     15.793 s ±  0.068 s    [User: 42.738 s, System: 15.383 s]
  Range (min … max):   15.670 s … 15.887 s    10 runs

Benchmark #4: ddh ~
  Time (mean ± σ):     10.032 s ±  0.062 s    [User: 36.698 s, System: 28.881 s]
  Range (min … max):    9.948 s … 10.136 s    10 runs

Benchmark #5: dupe-krill -s -d ~
  Time (mean ± σ):      8.396 s ±  0.068 s    [User: 5.323 s, System: 3.029 s]
  Range (min … max):    8.301 s …  8.504 s    10 runs

Benchmark #6: fddf -m 0 ~
  Time (mean ± σ):      5.146 s ±  0.047 s    [User: 10.320 s, System: 12.963 s]
  Range (min … max):    5.061 s …  5.216 s    10 runs

Benchmark #7: yadf ~
  Time (mean ± σ):      2.969 s ±  0.015 s    [User: 10.076 s, System: 13.813 s]
  Range (min … max):    2.958 s …  3.008 s    10 runs

Summary
  'yadf ~' ran
    1.24 ± 0.01 times faster than 'fclones --min-size 0 -R ~'
    1.73 ± 0.02 times faster than 'fddf -m 0 ~'
    2.83 ± 0.03 times faster than 'dupe-krill -s -d ~'
    3.38 ± 0.03 times faster than 'ddh ~'
    3.79 ± 0.02 times faster than 'jdupes -z -r ~'
    5.32 ± 0.04 times faster than 'rmlint --hidden ~'
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
