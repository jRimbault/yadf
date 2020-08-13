Other software:
- [fdupes 1.6.1](https://github.com/adrianlopezroche/fdupes)
- [jdupes 1.14.0](https://github.com/jbruchon/jdupes)
- [ddh master](https://github.com/darakian/ddh)
- [fddf 1.7.0](https://github.com/birkenfeld/fddf)
- [rmlint 2.9.0](https://github.com/sahib/rmlint)
- [dupe-krill 1.4.4](https://github.com/kornelski/dupe-krill)


Output of [`bench.sh`](bench.sh):

```
Benchmark #1: yadf ~
  Time (mean ± σ):      3.277 s ±  0.302 s    [User: 4.997 s, System: 5.842 s]
  Range (min … max):    2.888 s …  3.547 s    10 runs

Benchmark #2: fdupes -r ~
  Time (mean ± σ):     56.194 s ±  0.145 s    [User: 24.354 s, System: 31.784 s]
  Range (min … max):   55.883 s … 56.410 s    10 runs

Benchmark #3: jdupes -r ~
  Time (mean ± σ):      8.670 s ±  0.036 s    [User: 3.826 s, System: 4.787 s]
  Range (min … max):    8.621 s …  8.731 s    10 runs

Benchmark #4: ddh ~
  Time (mean ± σ):      5.623 s ±  0.044 s    [User: 23.375 s, System: 9.833 s]
  Range (min … max):    5.590 s …  5.734 s    10 runs

Benchmark #5: rmlint --hidden ~
  Time (mean ± σ):     14.058 s ±  0.078 s    [User: 39.138 s, System: 14.374 s]
  Range (min … max):   13.941 s … 14.223 s    10 runs

Benchmark #6: dupe-krill -s -d ~
  Time (mean ± σ):      7.835 s ±  0.066 s    [User: 4.935 s, System: 2.864 s]
  Range (min … max):    7.757 s …  7.946 s    10 runs

Benchmark #7: fddf ~
  Time (mean ± σ):      4.982 s ±  0.097 s    [User: 9.766 s, System: 12.687 s]
  Range (min … max):    4.818 s …  5.151 s    10 runs

Summary
  'yadf ~' ran
    1.52 ± 0.14 times faster than 'fddf ~'
    1.72 ± 0.16 times faster than 'ddh ~'
    2.39 ± 0.22 times faster than 'dupe-krill -s -d ~'
    2.65 ± 0.24 times faster than 'jdupes -r ~'
    4.29 ± 0.40 times faster than 'rmlint --hidden ~'
   17.15 ± 1.58 times faster than 'fdupes -r ~'
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
