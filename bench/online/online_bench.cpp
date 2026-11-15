// online_bench.cpp -- explicit MPC (RPOPT) against online QP solvers on the same closed-loop
// trajectories: DAQP (dual active set, warm and cold) and qpOASES (online active set strategy:
// homotopy from the previous parameter, i.e. hotstart; and a cold init per step).
//
// Input: <name>.bin from `cargo run --release --example export_online` (format in that file).
// Per-step time: minimum over R repetitions. A warm-started solve needs the state of the previous
// step, so each repetition first re-solves the previous step (untimed) and then times the current
// one. The first step of a trajectory has no previous step and is counted in the cold columns only.
// The explicit times come from the exporter (Rust: hash-table point location + first input).
// Every solver's first input is checked against the explicit law.
//
//   ./online_bench data/*.bin
#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>
extern "C" {
#include "api.h"
#include "auxiliary.h"  // daqp_deactivate_constraints
#include "utils.h"      // daqp_update_ldp
}
#include <qpOASES.hpp>

using std::vector;
static double now_ns() { return (double)std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now().time_since_epoch()).count(); }

struct Data {
  uint64_t nx, nt, m, nu, T, regions;
  vector<double> Q, c, H, A, b, F, th, uref, expl, start;
  double hash_bytes;
};

static bool load(const char* path, Data& d) {
  FILE* f = fopen(path, "rb");
  if (!f) return false;
  uint64_t h[7];
  if (fread(h, 8, 7, f) != 7 || h[0] != 0x4f4e4c494e45ull) { fclose(f); return false; }
  d.nx = h[1]; d.nt = h[2]; d.m = h[3]; d.nu = h[4]; d.T = h[5]; d.regions = h[6];
  auto rd = [&](vector<double>& v, size_t n) { v.resize(n); return fread(v.data(), 8, n, f) == n; };
  bool ok = rd(d.Q, d.nx * d.nx) && rd(d.c, d.nx) && rd(d.H, d.nx * d.nt) && rd(d.A, d.m * d.nx) && rd(d.b, d.m) && rd(d.F, d.m * d.nt) &&
            rd(d.th, d.T * d.nt) && rd(d.uref, d.T * d.nu) && rd(d.expl, d.T) && rd(d.start, d.T) && fread(&d.hash_bytes, 8, 1, f) == 1;
  fclose(f);
  return ok;
}

// g = c + H theta, ub = b + F theta
static void params(const Data& d, const double* th, double* g, double* ub) {
  for (size_t i = 0; i < d.nx; i++) { double s = d.c[i]; for (size_t j = 0; j < d.nt; j++) s += d.H[i * d.nt + j] * th[j]; g[i] = s; }
  for (size_t i = 0; i < d.m; i++) { double s = d.b[i]; for (size_t j = 0; j < d.nt; j++) s += d.F[i * d.nt + j] * th[j]; ub[i] = s; }
}

struct Stats { double med, p99, max, mean; };
static Stats stats(vector<double> v) {
  std::sort(v.begin(), v.end());
  double mean = 0; for (double x : v) mean += x; mean /= v.size();
  return {v[v.size() / 2], v[(size_t)(0.99 * (v.size() - 1))], v.back(), mean};
}

// ------------------------------------------------------------------ DAQP
struct Daqp {
  DAQPWorkspace work; DAQPProblem qp; vector<double> f, bu, bl, x, lam; vector<int> sense;
  Daqp(const Data& d) : f(d.nx), bu(d.m), bl(d.m, -1e30), x(d.nx), lam(d.m), sense(d.m, 0) {
    memset(&work, 0, sizeof(work)); memset(&qp, 0, sizeof(qp));
    params(d, &d.th[0], f.data(), bu.data());
    qp.n = (int)d.nx; qp.m = (int)d.m; qp.ms = 0; qp.H = (double*)d.Q.data(); qp.f = f.data(); qp.A = (double*)d.A.data();
    qp.bupper = bu.data(); qp.blower = bl.data(); qp.sense = sense.data();
    double st; allocate_daqp_settings(&work);
    int fl = setup_daqp_main(&qp, &work, &st, 0);
    if (fl < 0) { printf("daqp setup failed %d\n", fl); exit(1); }
  }
  int solve(const Data& d, const double* th, bool cold) {
    params(d, th, f.data(), bu.data());
    daqp_update_ldp(DAQP_UPDATE_v | DAQP_UPDATE_d, &work, &qp);
    if (cold) daqp_deactivate_constraints(&work);
    DAQPResult res = {x.data(), lam.data(), 0, 0, 0, 0, 0, 0, 0};
    daqp_solve(&res, &work);
    return res.exitflag > 0 ? res.iter : -1;
  }
};

// ------------------------------------------------------------------ qpOASES
struct Qpo {
  qpOASES::QProblem qp; vector<double> g, ub, lb, x; int nx, m; const Data* d; bool inited = false;
  Qpo(const Data& dd) : qp((int)dd.nx, (int)dd.m), g(dd.nx), ub(dd.m), lb(dd.m, -1e20), x(dd.nx), nx((int)dd.nx), m((int)dd.m), d(&dd) {
    qpOASES::Options o; o.setToMPC(); o.printLevel = qpOASES::PL_NONE; qp.setOptions(o);
  }
  int solve(const double* th, bool cold) {
    params(*d, th, g.data(), ub.data());
    qpOASES::int_t nwsr = 10 * (nx + m);
    qpOASES::returnValue rv;
    if (cold || !inited) {
      qp.reset();
      qpOASES::Options o; o.setToMPC(); o.printLevel = qpOASES::PL_NONE; qp.setOptions(o);
      rv = qp.init(d->Q.data(), g.data(), d->A.data(), nullptr, nullptr, lb.data(), ub.data(), nwsr);
      inited = true;
    } else {
      rv = qp.hotstart(g.data(), nullptr, nullptr, lb.data(), ub.data(), nwsr);
    }
    if (rv != qpOASES::SUCCESSFUL_RETURN) return -1;
    qp.getPrimalSolution(x.data());
    return (int)nwsr;
  }
};

template <class SolveAt, class First>
static void run(const Data& d, const char* name, bool warm, int R, SolveAt solve_at, First first, double& err, int& fails, vector<double>& t, vector<double>& it) {
  t.clear(); it.clear(); err = 0; fails = 0;
  for (size_t k = 0; k < d.T; k++) {
    if (warm && d.start[k] != 0.0) continue;
    double best = 1e300; int iters = 0;
    for (int r = 0; r < R; r++) {
      if (warm) solve_at(k - 1, false);
      double t0 = now_ns();
      iters = solve_at(k, !warm);
      double dt = now_ns() - t0;
      if (dt < best) best = dt;
    }
    if (iters < 0) { fails++; continue; }
    const double* u = first();
    for (size_t j = 0; j < d.nu; j++) err = std::max(err, fabs(u[j] - d.uref[k * d.nu + j]));
    t.push_back(best); it.push_back(iters);
  }
  (void)name;
}

int main(int argc, char** argv) {
  printf("%-12s %7s %3s %4s %4s | %-18s %9s %9s %9s %7s %9s %5s\n", "problem", "regions", "nt", "nx", "m", "method", "median", "p99", "max", "iters", "u err", "fail");
  for (int a = 1; a < argc; a++) {
    Data d;
    if (!load(argv[a], d)) { printf("cannot read %s\n", argv[a]); continue; }
    std::string nm = argv[a]; nm = nm.substr(nm.find_last_of('/') + 1); nm = nm.substr(0, nm.size() - 4);
    auto line = [&](const char* method, const vector<double>& t, const vector<double>& it, double err, int fails, const char* note) {
      Stats s = stats(t); double mi = 0; for (double x : it) mi += x; mi = it.empty() ? 0 : mi / it.size();
      printf("%-12s %7llu %3llu %4llu %4llu | %-18s %9.0f %9.0f %9.0f %7.1f %9.1e %5d %s\n", nm.c_str(), (unsigned long long)d.regions, (unsigned long long)d.nt,
             (unsigned long long)d.nx, (unsigned long long)d.m, method, s.med, s.p99, s.max, mi, err, fails, note);
      fflush(stdout);
    };
    vector<double> t, it; double err; int fails;
    {
      vector<double> e(d.expl), none(d.T, 0); char note[64]; snprintf(note, sizeof note, "(hash table %.2f MB)", d.hash_bytes / 1048576.0);
      line("explicit (RPOPT)", e, none, 0, 0, note);
    }
    Daqp dq(d);
    for (int warm = 1; warm >= 0; warm--) {
      run(d, "daqp", warm, 20, [&](size_t k, bool cold) { return dq.solve(d, &d.th[k * d.nt], cold); }, [&] { return (const double*)dq.x.data(); }, err, fails, t, it);
      line(warm ? "DAQP warm" : "DAQP cold", t, it, err, fails, "");
    }
    Qpo qo(d);
    run(d, "qpoases", true, 20, [&](size_t k, bool cold) { return qo.solve(&d.th[k * d.nt], cold); }, [&] { return (const double*)qo.x.data(); }, err, fails, t, it);
    line("qpOASES hotstart", t, it, err, fails, "");
    run(d, "qpoases", false, 5, [&](size_t k, bool cold) { return qo.solve(&d.th[k * d.nt], cold); }, [&] { return (const double*)qo.x.data(); }, err, fails, t, it);
    line("qpOASES cold", t, it, err, fails, "");
  }
}
