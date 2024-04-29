

// copied from profile.py
const allmodes = [["rvsdg_roundtrip", "rvsdg-round-trip-to-executable", ""],

["egglog_noopt_brilift_noopt", "compile-brilift", "--optimize-egglog false --optimize-brilift false"],
["egglog_noopt_brilift_opt", "compile-brilift", "--optimize-egglog false --optimize-brilift true"],
["egglog_opt_brilift_noopt", "compile-brilift", "--optimize-egglog true --optimize-brilift false"],
["egglog_opt_brilift_opt", "compile-brilift", "--optimize-egglog true --optimize-brilift true"],

["egglog_noopt_bril_llvm_noopt", "compile-bril-llvm", "--optimize-egglog false --optimize-bril-llvm false"],
["egglog_noopt_bril_llvm_opt", "compile-bril-llvm", "--optimize-egglog false --optimize-bril-llvm true"],
["egglog_opt_bril_llvm_noopt", "compile-bril-llvm", "--optimize-egglog true --optimize-bril-llvm false"],
["egglog_opt_bril_llvm_opt", "compile-bril-llvm", "--optimize-egglog true --optimize-bril-llvm true"]]


var enabledModes = new Set();

async function getPreviousRuns() {
    const req = await fetch("https://nightly.cs.washington.edu/reports-json/eggcc/");
    const files = await req.json();
    
    // map files into objects of the shape:
    // {branch: <git branch:string>, commit: <git commit:string>, date: <unix timestamp:int>, url: <absolute url to nightly page:string>}
    const comparisons = [];
    // start at i=1 because / is the first file
    for (let i = 1; i < files.length; i++) {
        // file name is of the format <date>:"nightly":<branch>:<commit>
        const [date, _, branch, commit] = files[i].name.split(":");

        const run = {
            branch: branch,
            commit: commit,
            // type coerce a unix timestamp that is a string into an int with a `+`
            date: +date,
            // The file server only hands us back the directory names,
            // but we want to make sure that we only use absolute URLs so that we can run this page
            // in any environment (local or otherwise)
            url: `https://nightly.cs.washington.edu/reports/eggcc/${files[i].name}`,
        }
        
        comparisons.push(run);
    }

    // sort runs in descending order
    comparisons.sort((l, r) => {
        if (l.date < r.date) { return 1; }
        if (l.date > r.date) { return -1; }
        return 0;
    });

    return comparisons.slice(0, comparisons.length < 30 ? comparisons.length : 30);
}

async function buildNightlyDropdown(element, previousRuns, initialIdx) {
    const select = document.getElementById(element);
    
    const formatRun = (run) => `${run.branch} - ${run.commit} - ${(new Date(run.date * 1000)).toDateString()}`
    
    
    previousRuns.forEach((nightly) => {
        console.log(nightly);

        const option = document.createElement("option");
        option.innerText = formatRun(nightly);
        select.appendChild(option);
    });
    
    select.onchange = () => {
        const compareTo = previousRuns[select.selectedIndex];
        loadBenchmarks(compareTo);
    }

    select.selectedIndex = initialIdx;
    select.value = formatRun(previousRuns[initialIdx]);
    select.onchange();
}

// findBenchToCompare will find the last benchmark run on the main branch that is not itself
function findBenchToCompareIdx(benchRuns) {
    // Determine what benchmark run we are based on the browser's URL
    // This is likely the best way to do this without embedding a bunch of data into our profile.js file
    // or our profile.json file, which although tempting, is not backwards compatible
    const path = window.location.pathname;
    const parts = path.split("/");

    // URLs should have trailing slashes leaving `parts` with a blank last element,
    // so we should index into `parts` at its length-2
    // Just in case the URL somehow doesn't have a trailing slash and `parts` doesn't
    // have a blank last element, do a quick check and adjust the index accordingly
    const idx = path[path.length - 1] === "/" ? parts.length - 2 : parts.length - 1;
    
    const [date, _, branch, commit] = parts[idx].split("%3A");
    for (let i = 0; i < benchRuns.length; i++) {
        const run = benchRuns[i];
        if (run.branch === "main") {
            // If we are comparing a run on a main branch, to previous main branch we need to make sure
            // it is not the same branch.
            // I did mean `==` here, not `===`. `curComparison.date` is an int, and `date` is a string
            if (branch === "main" && run.commit === commit && run.date == date) {
                continue; // skip, we're on the same branch
            }
            
            // the branch is now either the latest main run, or if on main the previous main run
            // return it
            return i;
        }
    }
    throw new Error("Couldn't find a benchmark run from main for comparison");
}

async function getBench(url) {
    const resp = await fetch(url + "data/profile.json");
    const benchData = await resp.json();
    return groupByBenchmark(benchData)
}

// benchList should be in the format of Array<{runMethod: String, benchmark: String, total_dyn_inst: Int, hyperfine: Array<{results: **hyperfine `--json` results**}>}>
function groupByBenchmark(benchList) {
    const groupedBy = {};
    benchList.forEach((obj) => {
        if (!groupedBy[obj.benchmark]) {
            groupedBy[obj.benchmark] = {};
        }
        groupedBy[obj.benchmark][obj.runMethod] = obj;
    });
    return groupedBy;
}

const compareKeys = ["# Instructions"];
function buildEntry(run) {
    const results = run.hyperfine.results[0];
    return {
        name: run.runMethod,
        mean: tryRound(results.mean),
        min: tryRound(results.min),
        max: tryRound(results.max),
        median: tryRound(results.median),
        stddev: tryRound(results.stddev),
    }
}

// TODO (@ryan-berger) decide how to compare to prevRun now there are no instruction count metrics
function buildTableText(prevRun, run) {
    const entry = buildEntry(run)
    if (!prevRun) {
        return entry;
    }
    return entry;
}

async function loadBenchmarks(compareTo) {
    const currentRun = await getBench("./");
    let previousRun = undefined;
    try {
        previousRun = await getBench(compareTo.url+"/");
    } catch (e) {}

    const benchmarkNames = Object.keys(currentRun);

    const parsed = benchmarkNames.map((benchName) => {
        return {
            name: benchName,
            "Executions ": {
                data: Object
                    .keys(currentRun[benchName])
                    .map((runMethod) => {
                        // if the mode is not enabled, skip it
                        if (!enabledModes.has(runMethod)) {
                            return undefined;
                        }
                        const prevBenchmark = previousRun ? previousRun[benchName] : undefined;
                        const prevRun = prevBenchmark ? prevBenchmark[runMethod] : undefined;
                        
                        return buildTableText(prevRun, currentRun[benchName][runMethod]) 
                    })
                    .filter((entry) => entry !== undefined)
            }
        }
    });

    parsed.sort((l, r) => {
        if (l.name < r.name) { return -1; }
        if (l.name > r.name) { return 1; }
        return 0;
    });

    let container = document.getElementById("profile");
    container.innerHTML = ConvertJsonToTable(parsed);
}

function makecheckbox(parent, mode) {
    // make a check box for enabling this mode
    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.id = mode;
    checkbox.checked = true;
    checkbox.onchange = () => {
        // if the checkbox is checked, add the mode to the set of enabled modes
        if (checkbox.checked) {
            enabledModes.add(mode);
        } else {
            // if the checkbox is unchecked, remove the mode from the set of enabled modes
            enabledModes.delete(mode);
        }
        // load everything again
        loadBenchmarks();
    }
    parent.appendChild(checkbox);
    // make a label for the checkbox
    const label = document.createElement("label");
    label.htmlFor = mode;
    label.innerText = mode;
    parent.appendChild(label);
    // make a line break
    parent.appendChild(document.createElement("br"));

    // add this to the set of enabled modes
    enabledModes.add(mode);
}

// Top-level load function for the main index page.
async function load() {
    // for each mode in allmodes, make a box to enable or disable it
    // put them in the div with id modeselections
    const modeSelections = document.getElementById("modeselections");
    allmodes.forEach((mode) => {
        makecheckbox(modeSelections, mode[0])
    })



    const previousRuns = await getPreviousRuns();
    const initialRunIdx = findBenchToCompareIdx(previousRuns);
    
    buildNightlyDropdown("comparison", previousRuns, initialRunIdx)
}
