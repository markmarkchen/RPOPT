"""ET-mp Benders on the FOCAPO six-process capacity-expansion case study, with the cache of
`rpopt.EventTriggered` in place of the case study's own numba active-set cacher.

The case study code is not copied here. This driver runs the case study's own script (master MILP,
lazy-cut callback, cut throttling, seed cuts, timers) from its folder and replaces only the class
`SubproblemSolver`, which answers a batch of subproblem parameters with (status, phi, Pi[:6]).

    python bench/et_benders_focapo.py --case-dir "<...>/6 process/code" --engine rpopt --scenarios 2000
    python bench/et_benders_focapo.py --case-dir ... --engine numba      # the script unchanged
    python bench/et_benders_focapo.py --case-dir ... --validate 2000     # subproblem answers vs Gurobi

Needs gurobipy (and numba for --engine numba) and the RPOPT bindings on the path, e.g.
PYTHONPATH=<RPOPT>/python.
"""

import argparse
import os
import re
import sys
import time

import numpy as np

SCRIPT = "on_the_fly_benders_as_12++.py"
SOLVE_LINE = "optimal_expected_cost = solve_benders_milp(data, A, b, F, c, H, equality_indices)"
N_Q = 6  # the first rows are d = q; their multipliers are the cut coefficients


def load_case(case_dir, script, scenarios, periods, seed=None):
    """The case study script up to the Benders run, executed; returns its namespace. `seed`
    replaces the script's `Randseed` (the scenario data generator's seed)."""
    import matplotlib

    matplotlib.use("Agg")
    sys.path.insert(0, case_dir)
    src = open(os.path.join(case_dir, script), encoding="utf8").read()
    src = re.sub(r"^Num_scenarios\s*=.*$", f"Num_scenarios={scenarios}", src, flags=re.M)
    src = re.sub(r"^Time_periods\s*=.*$", f"Time_periods={periods}", src, flags=re.M)
    if seed is not None:
        src = re.sub(r"^Randseed\s*=.*$", f"Randseed = {seed}", src, flags=re.M)
    if SOLVE_LINE not in src:
        raise SystemExit(f"{script}: cannot find the line {SOLVE_LINE!r}")
    head, _ = src.split(SOLVE_LINE, 1)
    g = {"__name__": "focapo_case"}
    cwd = os.getcwd()
    os.chdir(case_dir)
    try:
        exec(compile(head, os.path.join(case_dir, script), "exec"), g)
    finally:
        os.chdir(cwd)
    return g


class _Telemetry:
    """What the case study prints and plots from `_SUB_SOLVER.cacher`."""

    def __init__(self, et):
        self.et = et

    @property
    def cache(self):
        return [{"frequency": h} for h in self.et.region_hits().values()]

    @property
    def cache_size(self):
        return len(self.et)

    def print_detailed_telemetry(self):
        s = self.et.stats()
        q = max(1, s["queries"])
        print("\n" + "=" * 50)
        print("RPOPT EventTriggered cache")
        print("=" * 50)
        print(f"Subproblem queries              : {s['queries']}")
        print(f"  cache hits                    : {s['hits']} ({100 * s['hits'] / q:.3f}%)")
        print(f"    safe ball / stream / successor / scan : {s['ball_hits']} / {s['stream_hits']} / {s['successor_hits']} / {s['scan_hits']}")
        print(f"  solved (miss)                 : {s['built'] + s['duplicates'] + s['solved']} ({s['built']} new regions, {s['duplicates']} duplicates)")
        print(f"  infeasible                    : {s['infeasible']}")
        print(f"Regions tested per query        : {s['regions_tested'] / q:.3f}")
        print(f"Critical regions cached         : {s['regions']} ({s['rows']} rows, {s['reductions']} reduced)")
        if s["reported"]:
            print(f"  misses reported to Gurobi     : {s['reported']} (re-queries after each new region count again)")
        print(f"Basis rule                      : {self.et.basis} (fallbacks {s['rule_fallbacks']}, pareto switches at hits {s['pareto_switches']})")
        print("=" * 50 + "\n")


def make_rpopt_solver(g, parallel=True, max_regions=None, use_streams=True, miss_solver="rpopt", basis="auto"):
    """A drop-in `SubproblemSolver` for the case study script backed by rpopt.EventTriggered.

    miss_solver="rpopt": misses are solved by RPOPT (policy "exact"); the region's basis is
    chosen by `basis` from RPOPT's LP answer. miss_solver="gurobi": policy "report"; each miss is
    solved by Gurobi's dual simplex, as in the case study, and Gurobi's optimal basis is cached with
    `insert_active_set`, so degenerate subproblems give the same duals (cuts) as the case study.

    basis: "auto" (lex for this mpLP), "solver", "lex", or "pareto" (Magnanti-Wong multipliers for the cut coordinates q = theta[:6],
    scored at the core point q = Q_U / 2)."""
    import rpopt

    perf = g["perf"]

    class RpoptSubproblemSolver:
        def __init__(self, A, b, F, c, H_T, eq_indices):
            n_t = F.shape[1]
            # parameter box: the capacities and every scenario's data, with a margin (a parameter
            # outside the box would be reported infeasible)
            static = np.vstack(list(g["_STATIC_PARAMS_BATCHED"].values()))
            q_hi = float(np.max(g["data"]["Q_pt_U"]))
            lo = np.concatenate([np.zeros(N_Q), static.min(axis=0)])
            hi = np.concatenate([np.full(N_Q, q_hi), static.max(axis=0)])
            pad = 0.05 * (hi - lo) + 1e-3 * (1 + np.abs(hi))
            lo, hi = lo - pad, hi + pad
            A_t = np.vstack([np.eye(n_t), -np.eye(n_t)])
            b_t = np.concatenate([hi, -lo]).reshape(-1, 1)
            self.program = rpopt.MPLP_Program(A, b, c.reshape(-1, 1), H_T, A_t, b_t, F, list(eq_indices))
            # processed rows are scaled; multiplier of an original row = processed multiplier / scale
            PA, Pb, PF = (np.asarray(self.program.A), np.asarray(self.program.b).ravel(), np.asarray(self.program.F))
            self.rows, self.scale = [], []
            for i in range(N_Q):
                orig = np.concatenate([A[i], [b[i, 0]], F[i]])
                proc = np.concatenate([PA[i], [Pb[i]], PF[i]])
                nz = np.abs(proc) > 1e-12
                ratio = orig[nz] / proc[nz]
                if not (np.array_equal(np.abs(orig) > 1e-12, nz) and np.allclose(ratio, ratio[0])):
                    raise RuntimeError(f"row {i} moved during processing; map it explicitly")
                self.rows.append(i)
                self.scale.append(ratio[0])
            self.rows, self.scale = np.array(self.rows), np.array(self.scale)
            policy = "exact" if miss_solver == "rpopt" else "report"
            extra = {}
            if basis == "pareto":
                q_u = np.asarray(g["data"]["Q_pt_U"], float).ravel()[:N_Q]
                extra = {"cut_coords": list(range(N_Q)), "core": 0.5 * q_u}
            self.et = rpopt.EventTriggered(self.program, miss=policy, parallel=parallel, max_regions=max_regions, basis=basis, **extra)
            self.cacher = _Telemetry(self.et)
            self.use_streams = use_streams
            self._proj = np.random.default_rng(0).standard_normal(n_t - N_Q)
            self.gurobi = None
            if miss_solver == "gurobi":
                import gurobipy as gp

                self.gp = gp
                # the processed program has the same rows in the same order (checked above for the cut
                # rows); A, b, F, c, H_T of the original are used, as in the case study
                self.A, self.b, self.F, self.c, self.H_T = A, b.ravel(), F, c.ravel(), H_T
                m, nx = A.shape
                md = gp.Model("sub_rpopt_report")
                md.Params.OutputFlag = 0
                md.Params.Method = 1
                x = md.addMVar(nx, lb=-gp.GRB.INFINITY)
                eqs = set(eq_indices)
                self.cons = [md.addConstr(A[i] @ x == b[i, 0]) if i in eqs else md.addConstr(A[i] @ x <= b[i, 0]) for i in range(m)]
                self.gurobi, self.gx = md, x
                self.n_external = 0
                self.insert_failures = 0

        def _gurobi_solve(self, theta):
            gp = self.gp
            rhs = self.b + self.F @ theta
            self.gurobi.setObjective((self.c + self.H_T @ theta) @ self.gx, gp.GRB.MINIMIZE)
            for i, ci in enumerate(self.cons):
                ci.RHS = rhs[i]
            self.gurobi.optimize()
            if self.gurobi.Status != gp.GRB.OPTIMAL:
                return None
            pi = np.array([ci.Pi for ci in self.cons[:N_Q]])
            active = [i for i, ci in enumerate(self.cons) if ci.CBasis != 0]
            return self.gurobi.ObjVal, pi, active

        def _streams(self, theta_batch):
            # one stream per (scenario, period): the static part of theta identifies it exactly
            key = np.ascontiguousarray(theta_batch[:, N_Q:]) @ self._proj
            return (key.view(np.uint64) & np.uint64((1 << 63) - 1)).tolist()

        def solve_parametric_batched(self, theta_batch):
            theta_batch = np.ascontiguousarray(theta_batch, dtype=float)
            t0 = time.perf_counter()
            out = self.et.query_batch(theta_batch, self._streams(theta_batch) if self.use_streams else None)
            perf.record("rpopt_query_batch", time.perf_counter() - t0)
            ok = (out["status"] <= 2)  # hit, built, solved
            # Gurobi's Pi of a row of A x <= b in a minimisation is -(KKT multiplier)
            pi = -out["duals"][:, self.rows] / self.scale
            obj = out["objective"]
            bad = np.ones(N_Q) * 10.0
            res = [(0, obj[i], pi[i]) if ok[i] else (1, 0.0, bad) for i in range(len(obj))]
            if self.gurobi is None:
                return res
            # policy "report": solve the first miss with Gurobi, cache its basis, re-query the rest
            miss = np.flatnonzero(out["status"] == 4).tolist()
            streams = self._streams(theta_batch) if self.use_streams else None
            while miss:
                i = miss.pop(0)
                t1 = time.perf_counter()
                sol = self._gurobi_solve(theta_batch[i])
                perf.record("gurobi_solve", time.perf_counter() - t1)
                if sol is None:
                    res[i] = (1, 0.0, bad)
                    continue
                phi, pi_i, active = sol
                res[i] = (0, phi, pi_i)
                self.n_external += 1
                try:
                    self.et.insert_active_set(active)
                except ValueError:
                    self.insert_failures += 1
                    continue
                if miss:
                    t1 = time.perf_counter()
                    sub = self.et.query_batch(theta_batch[miss], [streams[j] for j in miss] if streams else None)
                    perf.record("rpopt_query_batch", time.perf_counter() - t1)
                    sub_pi = -sub["duals"][:, self.rows] / self.scale
                    left = []
                    for j, idx in enumerate(miss):
                        if sub["status"][j] == 0:
                            res[idx] = (0, sub["objective"][j], sub_pi[j])
                        else:
                            left.append(idx)
                    miss = left
            return res

        def solve_parametric(self, param_vector):
            return self.solve_parametric_batched(np.asarray(param_vector, float).reshape(1, -1))[0]

    return RpoptSubproblemSolver


def validate(g, n, seed=0):
    """Subproblem answers of the rpopt adapter against Gurobi at random (capacity, scenario) points:
    objective, and the cut alpha >= phi + Pi'(q' - q) at other capacities q' (Pi is a valid
    subgradient of phi in q even when it is not unique)."""
    import gurobipy as gp

    data = g["data"]
    # what solve_benders_milp precomputes, rebuilt here without the master
    T = list(data["T_set"])
    K = list(data["K_set"])
    sigma = np.array(data["sigma_pt"]).T
    static = {}
    for t in T:
        rows = []
        for k in K:
            s = data["scenarios"][k]
            gam, phi, Av, D = (np.array(s[x]).T[t] for x in ("gamma", "phi", "A", "D"))
            gi, pi_ = np.nonzero(gam), np.nonzero(phi)
            rows.append(np.hstack((gam[gi], sigma[t], phi[pi_], Av[pi_], D[gi])))
        static[t] = np.vstack(rows)
    g["_STATIC_PARAMS_BATCHED"] = static
    A, b, F, c, H = g["A"], g["b"], g["F"], g["c"], g["H"]
    eq = g["equality_indices"]
    solver = make_rpopt_solver(g, use_streams=False)(A, b, F, c, H.T, eq)

    m, nx = A.shape
    model = gp.Model()
    model.Params.OutputFlag = 0
    x = model.addMVar(nx, lb=-gp.GRB.INFINITY)
    cons = [model.addConstr(A[i] @ x == b[i, 0]) if i in eq else model.addConstr(A[i] @ x <= b[i, 0]) for i in range(m)]

    def gurobi(theta):
        rhs = b.ravel() + F @ theta
        model.setObjective((c.ravel() + H.T @ theta) @ x, gp.GRB.MINIMIZE)
        for i, ci in enumerate(cons):
            ci.RHS = rhs[i]
        model.optimize()
        return model.ObjVal, np.array([ci.Pi for ci in cons[:N_Q]])

    rng = np.random.default_rng(seed)
    q_hi = np.array(data["Q_pt_U"], float)
    worst_obj = worst_cut = 0.0
    t0 = time.perf_counter()
    thetas = []
    for _ in range(n):
        t = rng.choice(T)
        k = rng.integers(len(K))
        q = q_hi * rng.random(N_Q) * (rng.random(N_Q) < 0.8)
        thetas.append(np.concatenate([q, static[t][k]]))
    thetas = np.array(thetas)
    res = solver.solve_parametric_batched(thetas)
    t_rp = time.perf_counter() - t0
    t0 = time.perf_counter()
    ref = [gurobi(th) for th in thetas]
    t_gu = time.perf_counter() - t0
    for th, (st, phi, pi), (phi_g, pi_g) in zip(thetas, res, ref):
        assert st == 0, "rpopt reported a failed subproblem"
        worst_obj = max(worst_obj, abs(phi - phi_g) / (1 + abs(phi_g)))
        for _ in range(3):
            q2 = q_hi * rng.random(N_Q)
            th2 = th.copy()
            th2[:N_Q] = q2
            phi2, _ = gurobi(th2)
            # convexity in q: phi(q2) >= phi(q) + Pi'(q2 - q)
            worst_cut = max(worst_cut, (phi + pi @ (q2 - th[:N_Q]) - phi2) / (1 + abs(phi2)))
    s = solver.et.stats()
    print(f"validate: {n} random subproblems, {s['regions']} regions built, {s['hits']} hits")
    print(f"  worst relative objective difference vs Gurobi: {worst_obj:.2e}")
    print(f"  worst relative cut violation at other capacities: {worst_cut:.2e}  (<= 0 up to rounding means valid)")
    print(f"  rpopt batch {t_rp * 1e3:.1f} ms, Gurobi one by one {t_gu * 1e3:.1f} ms")
    return worst_obj < 1e-6 and worst_cut < 1e-6


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--case-dir", required=True, help="folder of the case study code (generate_problem.py, the script)")
    ap.add_argument("--script", default=SCRIPT)
    ap.add_argument("--engine", choices=["rpopt", "numba"], default="rpopt")
    ap.add_argument("--scenarios", type=int, default=2000)
    ap.add_argument("--periods", type=int, default=10)
    ap.add_argument("--seed", type=int, default=None, help="scenario data seed (the script's Randseed, 123)")
    ap.add_argument("--serial", action="store_true", help="rpopt: no rayon parallelism")
    ap.add_argument("--no-streams", action="store_true", help="rpopt: no stream ids")
    ap.add_argument("--max-regions", type=int, default=None)
    ap.add_argument("--miss-solver", choices=["rpopt", "gurobi"], default="rpopt",
                    help="rpopt: RPOPT solves misses and builds regions; gurobi: Gurobi solves misses, its basis is cached")
    ap.add_argument("--basis", choices=["auto", "solver", "lex", "pareto"], default="auto",
                    help="rpopt: which optimal basis a degenerate subproblem gets (see EventTriggered)")
    ap.add_argument("--validate", type=int, default=0, help="only check N subproblem answers against Gurobi")
    args = ap.parse_args()

    t0 = time.perf_counter()
    g = load_case(os.path.abspath(args.case_dir), args.script, args.scenarios, args.periods, args.seed)
    print(f"[driver] case loaded in {time.perf_counter() - t0:.1f} s")
    if args.validate:
        sys.exit(0 if validate(g, args.validate) else 1)
    if args.engine == "rpopt":
        g["SubproblemSolver"] = make_rpopt_solver(g, parallel=not args.serial, max_regions=args.max_regions,
                                                  use_streams=not args.no_streams, miss_solver=args.miss_solver, basis=args.basis)
    t0 = time.perf_counter()
    exec(SOLVE_LINE, g)
    wall = time.perf_counter() - t0
    g["_SUB_SOLVER"].cacher.print_detailed_telemetry()
    g["perf"].report()
    label = args.engine if args.engine == "numba" else f"rpopt (misses: {args.miss_solver}, basis: {args.basis})"
    print(f"\n[driver] engine={label} scenarios={args.scenarios} periods={args.periods} "
          f"objective={g['optimal_expected_cost']} solve_benders_milp wall={wall:.2f} s")


if __name__ == "__main__":
    main()
