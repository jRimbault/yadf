Other software:

- [fclones 0.8.0](https://github.com/pkolaczk/fclones)
- [jdupes 1.14.0](https://github.com/jbruchon/jdupes)
- [ddh 0.11.3](https://github.com/darakian/ddh)
- [fddf 1.7.0](https://github.com/birkenfeld/fddf)
- [rmlint 2.9.0](https://github.com/sahib/rmlint)
- [dupe-krill 1.4.4](https://github.com/kornelski/dupe-krill)

Output of [`bench.sh`](bench.sh):

```
Benchmark #1: fclones -R ~
  Time (mean ± σ):      3.648 s ±  0.035 s    [User: 15.911 s, System: 12.630 s]
  Range (min … max):    3.610 s …  3.717 s    10 runs

Benchmark #2: jdupes -z -r ~
  Time (mean ± σ):     10.941 s ±  0.038 s    [User: 5.597 s, System: 5.274 s]
  Range (min … max):   10.892 s … 10.999 s    10 runs

Benchmark #3: rmlint --hidden ~
  Time (mean ± σ):     16.009 s ±  0.098 s    [User: 42.302 s, System: 15.475 s]
  Range (min … max):   15.831 s … 16.202 s    10 runs

Benchmark #4: ddh ~
  Time (mean ± σ):     10.007 s ±  0.072 s    [User: 35.950 s, System: 28.828 s]
  Range (min … max):    9.831 s … 10.103 s    10 runs

Benchmark #5: dupe-krill -s -d ~
  Time (mean ± σ):      8.458 s ±  0.051 s    [User: 5.274 s, System: 3.139 s]
  Range (min … max):    8.381 s …  8.547 s    10 runs

Benchmark #6: fddf -m 0 ~
  Time (mean ± σ):      5.200 s ±  0.042 s    [User: 10.376 s, System: 13.075 s]
  Range (min … max):    5.119 s …  5.251 s    10 runs

Benchmark #7: yadf ~
  Time (mean ± σ):      2.984 s ±  0.021 s    [User: 10.272 s, System: 13.711 s]
  Range (min … max):    2.958 s …  3.025 s    10 runs

Summary
  'yadf ~' ran
    1.22 ± 0.01 times faster than 'fclones -R ~'
    1.74 ± 0.02 times faster than 'fddf -m 0 ~'
    2.83 ± 0.03 times faster than 'dupe-krill -s -d ~'
    3.35 ± 0.03 times faster than 'ddh ~'
    3.67 ± 0.03 times faster than 'jdupes -z -r ~'
    5.37 ± 0.05 times faster than 'rmlint --hidden ~'
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
