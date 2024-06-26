#!/usr/bin/env python3

import json
import os
import time
from glob import glob
from sys import stdout
import subprocess

import concurrent.futures

treatments = [
  "rvsdg-round-trip-to-executable",
  "cranelift-O3",
  "llvm-O0",
  "llvm-O0-eggcc",
  "llvm-O3",
  "llvm-O3-eggcc",
]

# Where to output files that are needed for nightly report
DATA_DIR = None

# Where to write intermediate files that should be cleaned up at the end of this script
TMP_DIR = "tmp"

def get_eggcc_options(run_mode, benchmark):
  llvm_out_dir = f"{DATA_DIR}/llvm/{benchmark}/{run_mode}"
  match run_mode:
    case "rvsdg-round-trip-to-executable":
      return f'--run-mode rvsdg-round-trip-to-executable --llvm-output-dir {llvm_out_dir}'
    case "cranelift-O3":
      return f'--run-mode cranelift --optimize-egglog false --optimize-brilift true'
    case "llvm-O0":
      return f'--run-mode llvm --optimize-egglog false --optimize-bril-llvm false --llvm-output-dir {llvm_out_dir}'
    case "llvm-O3":
      return f'--run-mode llvm --optimize-egglog false --optimize-bril-llvm true --llvm-output-dir {llvm_out_dir}'
    case "llvm-O0-eggcc":
      return f'--run-mode llvm --optimize-egglog true --optimize-bril-llvm false --llvm-output-dir {llvm_out_dir}'
    case "llvm-O3-eggcc":
      return f'--run-mode llvm --optimize-egglog true --optimize-bril-llvm true --llvm-output-dir {llvm_out_dir}'
    case _:
      raise Exception("Unexpected run mode: " + run_mode)
    

class Benchmark:
  def __init__(self, path, treatment, index, total):
    self.path = path
    self.name = path.split("/")[-1][:-len(".bril")]
    self.treatment = treatment
    # index of this benchmark (for printing)
    self.index = index
    # total number of benchmarks being run
    self.total = total

def benchmark_profile_dir(name):
  return f'{TMP_DIR}/{name}'

def setup_benchmark(name):
  profile_dir = benchmark_profile_dir(name)
  try:
    os.mkdir(profile_dir)
  except FileExistsError:
    print(f'{profile_dir} exists, overwriting contents')

def optimize(benchmark):
  print(f'[{benchmark.index}/{benchmark.total}] Optimizing {benchmark.name} with {benchmark.treatment}')
  profile_dir = benchmark_profile_dir(benchmark.name)
  cmd = f'cargo run --release {benchmark.path} {get_eggcc_options(benchmark.treatment, benchmark.name)} -o {profile_dir}/{benchmark.treatment}'
  print(f'Running: {cmd}')
  start = time.time()
  subprocess.call(cmd, shell=True)
  end = time.time()
  return (f"{profile_dir}/{benchmark.treatment}", end-start)



def bench(benchmark):
  print(f'[{benchmark.index}/{benchmark.total}] Benchmarking {benchmark.name} with {benchmark.treatment}')
  profile_dir = benchmark_profile_dir(benchmark.name)

  with open(f'{profile_dir}/{benchmark.treatment}-args') as f:
    args = f.read().rstrip()

    # check that we have a file for the benchmark
    if not os.path.isfile(f'{profile_dir}/{benchmark.treatment}'):
      # TODO add an error to the errors file
      #with open('nightly/data/errors.txt', 'a') as f:
        #f.write(f'ERROR: No executable found for {name} in {benchmark.path}\n')
      return None
    else:
      # TODO for final nightly results, remove `--max-runs 2` and let hyperfine find stable results
      cmd = f'hyperfine --style none --warmup 1 --max-runs 2 --export-json /dev/stdout "{profile_dir}/{benchmark.treatment}{" " + args if len(args) > 0 else ""}"'
      result = subprocess.run(cmd, capture_output=True, shell=True)
      return (f'{profile_dir}/{benchmark.treatment}', json.loads(result.stdout))

# Run modes that we expect to output llvm IR
def should_have_llvm_ir(runMethod):
  return runMethod in [
    "rvsdg-round-trip-to-executable",
    "llvm-O0",
    "llvm-O0-eggcc",
    "llvm-O3",
    "llvm-O3-eggcc",
  ]

# aggregate all profile info into a single json array.
def aggregate(compile_times, bench_times, benchmark_metadata):
    res = []

    for path in sorted(compile_times.keys()):
      name = path.split("/")[-2]
      runMethod = path.split("/")[-1]
      result = {"runMethod": runMethod, "benchmark": name, "hyperfine": bench_times[path], "compileTime": compile_times[path], "metadata": benchmark_metadata[name]}

      res.append(result)
    return res

def is_looped(bril_file):
  with open(bril_file) as f:
    txt = f.read()
    return "orig_main" in txt

if __name__ == '__main__':
  # expect two arguments
  if len(os.sys.argv) != 3:
    print("Usage: profile.py <bril_directory> <output_directory>")
    exit(1)

  # Create tmp directory for intermediate files
  try:
    os.mkdir(TMP_DIR)
  except FileExistsError:
    print(f"{TMP_DIR} exits, overwriting contents")

  bril_dir, DATA_DIR = os.sys.argv[1:]
  profiles = []
  # if it is a directory get all files
  if os.path.isdir(bril_dir):
    print(f'Running all bril files in {bril_dir}')
    profiles = glob(f'{bril_dir}/**/*.bril', recursive=True)
  else:
    profiles = [bril_dir]

  benchmark_metadata = {}
  for profile in profiles:
    name = profile.split("/")[-1][:-len(".bril")]
    benchmark_metadata[name] = {"looped": is_looped(profile), "path": profile}

  to_run = []
  index = 0
  total = len(profiles) * len(treatments)
  for benchmark_path in profiles:
    for treatment in treatments:
      to_run.append(Benchmark(benchmark_path, treatment, index, total))
      index += 1

  for benchmark in to_run:
    setup_benchmark(benchmark.name)
  

  compile_times = {}
  # create a thread pool for running optimization
  with concurrent.futures.ThreadPoolExecutor(max_workers = 6) as executor:
    futures = {executor.submit(optimize, benchmark) for benchmark in to_run}
    for future in concurrent.futures.as_completed(futures):
      (path, compile_time) = future.result()
      compile_times[path] = compile_time

  # running benchmarks sequentially for more reliable results
  # can set this to true for testing
  isParallelBenchmark = False

  bench_data = {}
  if isParallelBenchmark:
    # create a thread pool for running benchmarks
    with concurrent.futures.ThreadPoolExecutor(max_workers = 6) as executor:
      futures = {executor.submit(bench, benchmark) for benchmark in to_run}
      for future in concurrent.futures.as_completed(futures):
        res = future.result()
        if res is None:
          continue
        (path, _bench_data) = res
        bench_data[path] = _bench_data
  else:
    for benchmark in to_run:
      res = bench(benchmark)
      if res is None:
        continue
      (path, _bench_data) = res
      bench_data[path] = _bench_data

  nightly_data = aggregate(compile_times, bench_data, benchmark_metadata)
  with open(f"{DATA_DIR}/profile.json", "w") as profile:
    json.dump(nightly_data, profile, indent=2)

  # Clean up intermediate files
  os.system(f"rm -rf {TMP_DIR}")
