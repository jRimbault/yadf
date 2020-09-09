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
  Time (mean ± σ):     57.283 s ±  0.153 s    [User: 25.219 s, System: 32.011 s]
  Range (min … max):   57.013 s … 57.496 s    10 runs

Benchmark #2: jdupes -z -r ~
  Time (mean ± σ):      9.986 s ±  0.040 s    [User: 4.963 s, System: 4.968 s]
  Range (min … max):    9.899 s … 10.045 s    10 runs

Benchmark #3: rmlint --hidden ~
  Time (mean ± σ):     15.354 s ±  0.055 s    [User: 40.894 s, System: 15.305 s]
  Range (min … max):   15.264 s … 15.433 s    10 runs

Benchmark #4: ddh ~
  Time (mean ± σ):      9.454 s ±  0.071 s    [User: 34.400 s, System: 27.332 s]
  Range (min … max):    9.356 s …  9.569 s    10 runs

Benchmark #5: dupe-krill -s -d ~
  Time (mean ± σ):      7.919 s ±  0.062 s    [User: 5.002 s, System: 2.880 s]
  Range (min … max):    7.857 s …  8.063 s    10 runs

Benchmark #6: fddf -m 0 ~
  Time (mean ± σ):      5.076 s ±  0.031 s    [User: 9.800 s, System: 12.766 s]
  Range (min … max):    5.026 s …  5.114 s    10 runs

Benchmark #7: yadf ~
  Time (mean ± σ):      3.755 s ±  0.019 s    [User: 6.158 s, System: 7.101 s]
  Range (min … max):    3.732 s …  3.796 s    10 runs

Summary
  'yadf ~' ran
    1.35 ± 0.01 times faster than 'fddf -m 0 ~'
    2.11 ± 0.02 times faster than 'dupe-krill -s -d ~'
    2.52 ± 0.02 times faster than 'ddh ~'
    2.66 ± 0.02 times faster than 'jdupes -z -r ~'
    4.09 ± 0.03 times faster than 'rmlint --hidden ~'
   15.25 ± 0.09 times faster than 'fdupes -r ~'
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
