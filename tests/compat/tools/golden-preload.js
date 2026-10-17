
// ---- golden-data automation, appended to preload.js by `cargo run --bin golden` ----
// Reads every `<name>.txt` of $GOLDEN_IN, runs it in CircuitJS1 until the simulated times in
// $GOLDEN_TIMES (comma separated seconds) and writes `<name>.json` into $GOLDEN_OUT with the
// voltage across, current through and post voltages of every element at each time. When the
// wall-clock limit $GOLDEN_LIMIT_MS passes first, the file is written with `"timeout": true`.
// With $GOLDEN_STEPS (comma separated step counts) the checkpoints are that many time steps of
// each circuit instead: circuits with picosecond steps never reach a millisecond.
if (process.env.GOLDEN_IN) {
  const path = require('path');
  const IN = process.env.GOLDEN_IN, OUT = process.env.GOLDEN_OUT;
  const ONLY = process.env.GOLDEN_ONLY ? process.env.GOLDEN_ONLY.split(',') : null;
  const TIMES = (process.env.GOLDEN_TIMES || '0.001,0.01,0.05').split(',').map(Number);
  const STEPS = process.env.GOLDEN_STEPS ? process.env.GOLDEN_STEPS.split(',').map(Number) : null;
  const LIMIT_MS = Number(process.env.GOLDEN_LIMIT_MS || 20000);
  function log(m) { try { fs.appendFileSync(path.join(OUT, '_log.txt'), m + '\n'); } catch (e) {} }
  window.addEventListener('error', e => log('error: ' + e.message));
  window.alert = function (m) { log('alert: ' + m); };
  window.confirm = function () { return true; };
  window.oncircuitjsloaded = function (sim) {
    let files = fs.readdirSync(IN).filter(f => f.endsWith('.txt')).sort();
    if (ONLY) files = files.filter(f => ONLY.includes(f.replace(/\.txt$/, '')));
    if (process.env.GOLDEN_BENCH_MS) {
      // speed test: run each circuit flat out and count how far simulated time gets in a
      // stretch of wall-clock time, after a warm-up
      const BENCH_MS = Number(process.env.GOLDEN_BENCH_MS);
      const WARM_MS = 1500;
      let bi = 0;
      const nextBench = () => {
        if (bi >= files.length) { ipcRenderer.send('golden-done'); return; }
        const f = files[bi++];
        const name = f.replace(/\.txt$/, '');
        log('bench ' + f);
        sim.setSimRunning(false);
        sim.importCircuit(fs.readFileSync(path.join(IN, f), 'utf8'), false);
        sim.setSimRunning(true);
        setTimeout(() => {
          const t0 = sim.getTime(), w0 = Date.now();
          setTimeout(() => {
            const t1 = sim.getTime(), w1 = Date.now();
            sim.setSimRunning(false);
            const seconds = (w1 - w0) / 1000, ts = sim.getTimeStep();
            const result = { name, simSeconds: t1 - t0, wallSeconds: seconds, timeStep: ts,
                             stepsPerSecond: (t1 - t0) / ts / seconds };
            fs.writeFileSync(path.join(OUT, name + '.json'), JSON.stringify(result));
            setTimeout(nextBench, 50);
          }, BENCH_MS);
        }, WARM_MS);
      };
      setTimeout(nextBench, 500);
      return;
    }
    let idx = 0, cur = null;
    function snapshot() {
      return {
        time: sim.getTime(),
        timeStep: sim.getTimeStep(),
        elements: sim.getElements().map(e => {
          const posts = [];
          for (let k = 0; k < e.getPostCount(); k++) posts.push(e.getVoltage(k));
          return { type: e.getType(), voltageDiff: e.getVoltageDiff(), current: e.getCurrent(), posts };
        })
      };
    }
    function finish(timeout) {
      if (!cur) return;
      clearTimeout(cur.timer);
      sim.setSimRunning(false);
      cur.result.timeout = !!timeout;
      fs.writeFileSync(path.join(OUT, cur.name + '.json'), JSON.stringify(cur.result));
      cur = null;
      setTimeout(next, 50);
    }
    function next() {
      if (idx >= files.length) { ipcRenderer.send('golden-done'); return; }
      const f = files[idx++];
      const name = f.replace(/\.txt$/, '');
      log('circuit ' + f);
      cur = { name, want: 0, times: TIMES, result: { name, checkpoints: [] } };
      try {
        sim.setSimRunning(false);
        sim.importCircuit(fs.readFileSync(path.join(IN, f), 'utf8'), false);
        sim.setSimRunning(true);
      } catch (e) { cur.result.error = String(e); finish(false); return; }
      cur.timer = setTimeout(() => finish(true), LIMIT_MS);
    }
    sim.ontimestep = function () {
      if (!cur) return;
      if (STEPS && cur.times === TIMES) cur.times = STEPS.map(n => n * sim.getTimeStep());
      const t = sim.getTime();
      while (cur.want < cur.times.length && t >= cur.times[cur.want]) {
        cur.result.checkpoints.push(snapshot());
        cur.want++;
      }
      if (cur.want >= cur.times.length) finish(false);
    };
    setTimeout(next, 500);
  };
}
