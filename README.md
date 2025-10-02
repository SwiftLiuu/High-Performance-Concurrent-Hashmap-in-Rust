# Design and Implementation of a High-Performance Concurrent Hash Table

## Team Members
| Name           | Email                        |
|----------------|------------------------------|
| Xiangyu Liu    | swift.liu@mail.utoronto.ca   |
| Yilin Huai     | yilin.huai@mail.utoronto.ca  |


## Motivation
Concurrent hash tables are a fundamental building block in systems such as caches, databases, schedulers, and real-time analytics pipelines. In Rust, existing solutions (`DashMap`, `RwLock<HashMap>`) focus on usability or basic thread-safety, but they lack **predictable performance** under heavy, mixed workloads.  

Our project is motivated by the need for a **high-performance concurrent hash table in Rust** that:
- Supports efficient multi-threaded access to shared key-value pairs  
- Provides reproducible benchmarks and evaluation results  
- Explores both lock-based and lock-free synchronization techniques  
- Optionally investigates Hardware Transactional Memory (HTM) as a hardware-assisted optimization  

---

## Objective and Key Features
**Objective:**  
Implement a concurrent hash table in Rust that achieves **high throughput and stable tail latency** across diverse workloads (read-heavy, write-heavy, and mixed).  

**Core Methods**
- `set(key, value)` – insert or update a key-value pair  
- `get(key)` – retrieve a value  
- `delete(key)` – remove a key-value pair  
- `resize()` – dynamically expand the table  

**Key Features**
1. **Synchronization Strategies**
   - Fine-grained locks (`parking_lot`) and reader-writer locks  
   - Lock-free path with atomic operations and epoch-based reclamation  
   - Optional HTM-based fast path with lock fallback  
2. **Algorithmic Design**
   - Open addressing with Robin Hood or Cuckoo Hashing for predictable probe lengths  
   - Gradual, per-shard resizing to avoid global pauses  
3. **Workload Adaptation**
   - Support both integer and string keys  
   - Optimize for mixed and skewed workloads (e.g., Zipf distributions)  
4. **Observability**
   - Collect metrics: throughput, p50/p95/p99 latency, probe length, resize frequency, HTM abort rates  
   - Provide reproducible benchmark scripts with CSV + plots  

---

## Tentative Plan
**Team Size:** 2 (Xiangyu Liu & Yilin Huai)

**Week 1–2: Baseline**
- Implement core operations (`set/get/delete/resize`) with fine-grained locking  
- Add unit tests and property tests  

**Week 3–4: Advanced Synchronization**
- Implement Robin Hood / Cuckoo probing  
- Add gradual resize strategy  
- Prototype lock-free version with CAS + epoch-based reclamation  

**Week 5: HTM Exploration (Optional)**
- Experiment with HTM transactions (`XBEGIN`/`XEND`) for small critical sections  
- Implement retry + fallback to locks  
- Collect statistics on commit vs abort rates  

**Week 6–7: Benchmarking**
- Benchmark under diverse workloads (uniform, skewed, read-heavy, write-heavy, mixed)  
- Compare against `RwLock<HashMap>` and `DashMap`  
- Collect throughput, latency, and probe metrics  

**Week 8: Finalization**
- Optimize hot paths for cache efficiency  
- Prepare reproducible report and scripts  
- Record video slide presentation and demo  

**Roles**
- Xiangyu Liu: Core operations and probing algorithms  
- Yilin Huai: Synchronization methods and HTM branch  
- Both members are responsible for benchmarking, metrics, documentation 

---

## Novelty
- **Synchronization comparison:** Lock-based vs lock-free vs hardware-assisted (HTM) in Rust  
- **Tail latency focus:** Emphasis on p95/p99 latency, not just throughput  
- **Algorithmic innovation:** Adapt concurrent cuckoo/Robin Hood hashing into Rust’s safe memory model  
- **Hardware awareness:** Evaluate HTM’s potential as a fast path with lock fallback  
- **Workload generality:** Extend beyond integers to variable-length string keys for real-world workloads  

---

## Related Work

- Xiaozhou Li, David G. Andersen, Michael Kaminsky, Michael J. Freedman.  
  *Algorithmic Improvements for Fast Concurrent Cuckoo Hashing*.  
  In *Proceedings of EuroSys ’14*, Amsterdam, Netherlands, April 2014. ACM, pp. 27–40.  
  DOI: [10.1145/2592798.2592820](https://doi.org/10.1145/2592798.2592820)

- Josh Triplett, Paul E. McKenney, Jonathan Walpole.  
  *Resizable, Scalable, Concurrent Hash Tables via Relativistic Programming*.  
  In *Proceedings of the Linux Symposium / USENIX OLS (ATC 2011)*, July 2010, Ottawa, Canada, pp. 71–80.  
  [PDF Link](https://www.usenix.org/legacy/event/atc11/tech/final_files/Triplett.pdf)


- Tobias Maier, Peter Sanders, and Roman Dementiev.  
  *Concurrent Hash Tables: Fast and General(?)!*  
  *ACM Transactions on Parallel Computing*, Vol. 5, No. 4, Article 16, February 2019, pp. 1–32.  
  DOI: [10.1145/3309206](https://doi.org/10.1145/3309206)

- Zhiwen Chen, Xin He, Jianhua Sun, Hao Chen, Ligang He.  
  *Concurrent Hash Tables on Multicore Machines: Comparison, Evaluation and Implications*.  
  *Future Generation Computer Systems*, Vol. 82, 2018, pp. 127–141. Elsevier.  
  DOI: [10.1016/j.future.2017.12.054](https://doi.org/10.1016/j.future.2017.12.054) :contentReference[oaicite:0]{index=0}

- Alexey A. Paznikov, Vadim A. Smirnov, Artur R. Omelnichenko.  
  *Towards Efficient Implementation of Concurrent Hash Tables and Search Trees Based on Software Transactional Memory*.  
  In *Proceedings of FarEastCon 2019*, IEEE, pp. 1–6.  
  DOI: [10.1109/FarEastCon.2019.8934131](https://doi.org/10.1109/FarEastCon.2019.8934131) :contentReference[oaicite:1]{index=1}

- Julian Shun, Guy E. Blelloch.  
  *Phase-Concurrent Hash Tables for Determinism*.  
  In *Proceedings of the 26th ACM Symposium on Parallelism in Algorithms and Architectures (SPAA ’14)*, Prague, Czech Republic, June 2014, pp. 96–105. ACM.  
  DOI: [10.1145/2612669.2612687](https://doi.org/10.1145/2612669.2612687) :contentReference[oaicite:2]{index=2}
 

---

## Deliverables
- **Source Code**: Rust library crate with full implementation and documentation  
- **README.md**: Motivation, methodology, and reproducibility instructions  
- **Video Slide Presentation (5–10 min)**: Design overview and results  
- **Video Demo (3–10 min)**: Live benchmark run and metrics output  
- **Final Report**: Implementation, experiments, evaluation, and discussion  

---
