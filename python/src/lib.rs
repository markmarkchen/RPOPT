//! Python bindings for RPOPT. The native module is `rpopt._rpopt`; the pure Python package `rpopt`
//! re-exports it under PPOPT's module layout (`rpopt.mpqp_program`, `rpopt.mp_solvers.solve_mpqp`, ...).

use numpy::{
    ndarray::{Array1, Array2},
    IntoPyArray, PyArray1, PyArray2, PyArrayLike2, PyArrayLikeDyn, PyReadonlyArray2,
    AllowTypeChange, PyUntypedArrayMethods,
};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyList;
use rayon::prelude::*;
use rpopt::linalg::Mat;
use rpopt::point_location as pl;
use rpopt::point_location::PointLocator;
use rpopt::{MpProgram, MpqpAlgorithm, SolveOptions};
use std::sync::Arc;

// ---------------------------------------------------------------------------------------------
// conversions
// ---------------------------------------------------------------------------------------------

fn mat_from(a: &PyArrayLike2<'_, f64, AllowTypeChange>) -> Mat {
    let v = a.as_array();
    let (r, c) = v.dim();
    let mut data = Vec::with_capacity(r * c);
    for i in 0..r {
        for j in 0..c {
            data.push(v[[i, j]]);
        }
    }
    Mat::from_vec(r, c, data)
}

fn vec_from(a: &PyArrayLikeDyn<'_, f64, AllowTypeChange>) -> Vec<f64> {
    a.as_array().iter().cloned().collect()
}

fn mat_to<'py>(py: Python<'py>, m: &Mat) -> Bound<'py, PyArray2<f64>> {
    Array2::from_shape_vec((m.rows, m.cols), m.data.clone()).unwrap().into_pyarray(py)
}

fn col_to<'py>(py: Python<'py>, v: &[f64]) -> Bound<'py, PyArray2<f64>> {
    Array2::from_shape_vec((v.len(), 1), v.to_vec()).unwrap().into_pyarray(py)
}

fn flat_to<'py>(py: Python<'py>, v: &[f64]) -> Bound<'py, PyArray1<f64>> {
    Array1::from(v.to_vec()).into_pyarray(py)
}

/// A parameter point as the user gave it: (n_t,) stays 1-D, (n_t, 1) or (1, n_t) is a column.
struct Theta {
    v: Vec<f64>,
    column: bool,
}

fn theta_from(a: &PyArrayLikeDyn<'_, f64, AllowTypeChange>) -> Theta {
    Theta { v: vec_from(a), column: a.ndim() == 2 }
}

fn out_like<'py>(py: Python<'py>, v: &[f64], column: bool) -> Bound<'py, PyAny> {
    if column {
        col_to(py, v).into_any()
    } else {
        flat_to(py, v).into_any()
    }
}

fn check_dim(got: usize, want: usize) -> PyResult<()> {
    if got != want {
        return Err(PyValueError::new_err(format!("theta has {got} entries, the program has {want} parameters")));
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// SolverOutput
// ---------------------------------------------------------------------------------------------

#[pyclass(module = "rpopt._rpopt", frozen)]
pub struct SolverOutput {
    #[pyo3(get)]
    obj: f64,
    #[pyo3(get)]
    sol: Py<PyArray1<f64>>,
    #[pyo3(get)]
    slack: Py<PyArray1<f64>>,
    #[pyo3(get)]
    active_set: Vec<usize>,
    #[pyo3(get)]
    dual: Py<PyArray1<f64>>,
}

fn solver_output(py: Python<'_>, s: &rpopt::solver::SolverOutput) -> SolverOutput {
    SolverOutput {
        obj: s.obj,
        sol: flat_to(py, &s.sol).unbind(),
        slack: flat_to(py, &s.slack).unbind(),
        active_set: s.active_set.clone(),
        dual: flat_to(py, &s.dual).unbind(),
    }
}

#[pymethods]
impl SolverOutput {
    fn __repr__(&self, py: Python<'_>) -> String {
        format!("SolverOutput(obj={}, sol={}, active_set={:?})", self.obj, self.sol.bind(py), self.active_set)
    }
}

// ---------------------------------------------------------------------------------------------
// programs
// ---------------------------------------------------------------------------------------------

#[pyclass(subclass, module = "rpopt._rpopt", frozen, name = "MPLP_Program")]
pub struct MplpProgram {
    inner: Arc<MpProgram>,
}

#[pyclass(extends = MplpProgram, module = "rpopt._rpopt", frozen, name = "MPQP_Program")]
pub struct MpqpProgram;

type M<'a> = PyArrayLike2<'a, f64, AllowTypeChange>;
type V<'a> = PyArrayLikeDyn<'a, f64, AllowTypeChange>;

fn warn(py: Python<'_>, msg: &str) {
    if let Ok(w) = py.import("warnings") {
        let _ = w.call_method1("warn", (msg, py.get_type::<pyo3::exceptions::PyUserWarning>()));
    }
}

#[pymethods]
impl MplpProgram {
    #[new]
    #[pyo3(signature = (A, b, c, H, A_t, b_t, F, equality_indices=None))]
    #[allow(non_snake_case, clippy::too_many_arguments)]
    fn new(
        py: Python<'_>,
        A: M<'_>,
        b: V<'_>,
        c: V<'_>,
        H: M<'_>,
        A_t: M<'_>,
        b_t: V<'_>,
        F: M<'_>,
        equality_indices: Option<Vec<usize>>,
    ) -> PyResult<Self> {
        let (a, h, a_t, f) = (mat_from(&A), mat_from(&H), mat_from(&A_t), mat_from(&F));
        let (b, c, b_t) = (vec_from(&b), vec_from(&c), vec_from(&b_t));
        check_shapes(&a, &b, &c, &h, None, &a_t, &b_t, &f)?;
        let eq = equality_indices.unwrap_or_default();
        let p = py.detach(|| MpProgram::new_mplp(a, b, c, h, a_t, b_t, f, eq));
        for w in &p.warnings {
            warn(py, w);
        }
        Ok(MplpProgram { inner: Arc::new(p) })
    }
}

#[allow(clippy::too_many_arguments)]
fn check_shapes(a: &Mat, b: &[f64], c: &[f64], h: &Mat, q: Option<&Mat>, a_t: &Mat, b_t: &[f64], f: &Mat) -> PyResult<()> {
    let bad = |m: String| Err(PyValueError::new_err(m));
    if b.len() != a.rows {
        return bad(format!("b has {} entries but A has {} rows", b.len(), a.rows));
    }
    if c.len() != a.cols {
        return bad(format!("c has {} entries but A has {} columns", c.len(), a.cols));
    }
    if f.rows != a.rows {
        return bad(format!("F has {} rows but A has {} rows", f.rows, a.rows));
    }
    if h.rows != a.cols || h.cols != f.cols {
        return bad(format!("H is {}x{}, expected {}x{}", h.rows, h.cols, a.cols, f.cols));
    }
    if a_t.cols != f.cols || b_t.len() != a_t.rows {
        return bad("A_t, b_t and F disagree on the number of parameters".into());
    }
    if let Some(q) = q {
        if q.rows != a.cols || q.cols != a.cols {
            return bad(format!("Q is {}x{}, expected {}x{}", q.rows, q.cols, a.cols, a.cols));
        }
    }
    Ok(())
}

#[pymethods]
impl MpqpProgram {
    #[new]
    #[pyo3(signature = (A, b, c, H, Q, A_t, b_t, F, equality_indices=None))]
    #[allow(non_snake_case, clippy::too_many_arguments)]
    fn new(
        py: Python<'_>,
        A: M<'_>,
        b: V<'_>,
        c: V<'_>,
        H: M<'_>,
        Q: M<'_>,
        A_t: M<'_>,
        b_t: V<'_>,
        F: M<'_>,
        equality_indices: Option<Vec<usize>>,
    ) -> PyResult<(Self, MplpProgram)> {
        let (a, h, q, a_t, f) = (mat_from(&A), mat_from(&H), mat_from(&Q), mat_from(&A_t), mat_from(&F));
        let (b, c, b_t) = (vec_from(&b), vec_from(&c), vec_from(&b_t));
        check_shapes(&a, &b, &c, &h, Some(&q), &a_t, &b_t, &f)?;
        let eq = equality_indices.unwrap_or_default();
        let p = py.detach(|| MpProgram::new_mpqp(a, b, c, h, q, a_t, b_t, f, eq));
        for w in &p.warnings {
            warn(py, w);
        }
        Ok((MpqpProgram, MplpProgram { inner: Arc::new(p) }))
    }
}

#[pymethods]
impl MplpProgram {
    fn num_x(&self) -> usize {
        self.inner.num_x()
    }
    fn num_t(&self) -> usize {
        self.inner.num_t()
    }
    fn num_constraints(&self) -> usize {
        self.inner.num_constraints()
    }
    fn num_inequality_constraints(&self) -> usize {
        self.inner.num_inequality_constraints()
    }
    fn num_equality_constraints(&self) -> usize {
        self.inner.num_equality_constraints()
    }

    #[getter(A)]
    fn get_a<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        mat_to(py, &self.inner.a)
    }
    #[getter]
    fn b<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        col_to(py, &self.inner.b)
    }
    #[getter]
    fn c<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        col_to(py, &self.inner.c)
    }
    #[getter(H)]
    fn get_h<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        mat_to(py, &self.inner.h)
    }
    #[getter(Q)]
    fn get_q<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyArray2<f64>>> {
        self.inner.q.as_ref().map(|q| mat_to(py, q))
    }
    #[getter(A_t)]
    fn get_a_t<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        mat_to(py, &self.inner.a_t)
    }
    #[getter(b_t)]
    fn get_b_t<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        col_to(py, &self.inner.b_t)
    }
    #[getter(F)]
    fn get_f<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        mat_to(py, &self.inner.f)
    }
    #[getter]
    fn c_c(&self) -> f64 {
        self.inner.c_c
    }
    #[getter]
    fn c_t<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        col_to(py, &self.inner.c_t)
    }
    #[getter(Q_t)]
    fn get_q_t<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        mat_to(py, &self.inner.q_t)
    }
    #[getter]
    fn equality_indices(&self) -> Vec<usize> {
        self.inner.equality_indices.clone()
    }

    /// RPOPT processes constraints (equality reordering, scaling, redundancy removal) on
    /// construction, so this is a no-op kept for PPOPT script compatibility.
    fn process_constraints(&self) {}

    fn warnings(&self) -> Vec<String> {
        self.inner.warnings.clone()
    }

    fn evaluate_objective(&self, x: V<'_>, theta: V<'_>) -> PyResult<f64> {
        let (x, t) = (vec_from(&x), vec_from(&theta));
        check_dim(t.len(), self.inner.num_t())?;
        check_dim(x.len(), self.inner.num_x()).map_err(|_| PyValueError::new_err("x has the wrong length"))?;
        Ok(self.inner.evaluate_objective(&x, &t))
    }

    fn valid_parameter_realization(&self, theta: V<'_>) -> PyResult<bool> {
        let t = vec_from(&theta);
        check_dim(t.len(), self.inner.num_t())?;
        Ok(self.inner.valid_parameter_realization(&t))
    }

    /// Solves the program with theta fixed. None if infeasible or theta is outside A_t theta <= b_t.
    fn solve_theta(&self, py: Python<'_>, theta: V<'_>) -> PyResult<Option<SolverOutput>> {
        let t = vec_from(&theta);
        check_dim(t.len(), self.inner.num_t())?;
        let p = self.inner.clone();
        let out = py.detach(|| p.solve_theta_uncounted(&t));
        Ok(out.map(|s| solver_output(py, &s)))
    }

    fn solve_theta_variable(&self, py: Python<'_>) -> Option<SolverOutput> {
        let p = self.inner.clone();
        py.detach(|| p.solve_theta_variable()).map(|s| solver_output(py, &s))
    }

    /// Chebyshev ball of the (x, theta) feasible space; `sol` holds [x, theta, radius].
    fn feasible_space_chebychev_ball(&self, py: Python<'_>) -> Option<SolverOutput> {
        let p = self.inner.clone();
        py.detach(|| p.feasible_space_chebychev_ball()).map(|s| solver_output(py, &s.into()))
    }

    fn feasible_theta_point<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyArray2<f64>>> {
        let p = self.inner.clone();
        py.detach(|| p.feasible_theta_point()).map(|v| col_to(py, &v))
    }

    fn check_active_set_rank(&self, active_set: Vec<usize>) -> bool {
        self.inner.check_active_set_rank(&active_set)
    }

    fn check_feasibility(&self, py: Python<'_>, active_set: Vec<usize>, check_rank: Option<bool>) -> bool {
        let p = self.inner.clone();
        py.detach(|| p.check_feasibility(&active_set, check_rank.unwrap_or(true)))
    }

    fn latex(&self) -> Vec<String> {
        self.inner.latex()
    }

    fn __repr__(&self) -> String {
        format!(
            "{}(num_x={}, num_t={}, num_constraints={}, equalities={})",
            if self.inner.is_qp() { "MPQP_Program" } else { "MPLP_Program" },
            self.inner.num_x(),
            self.inner.num_t(),
            self.inner.num_constraints(),
            self.inner.num_equality_constraints()
        )
    }
}

fn program_of(obj: &Bound<'_, PyAny>) -> PyResult<Arc<MpProgram>> {
    let b = obj
        .downcast::<MplpProgram>()
        .map_err(|_| PyValueError::new_err("expected an MPLP_Program or MPQP_Program"))?;
    Ok(b.get().inner.clone())
}

// ---------------------------------------------------------------------------------------------
// critical regions
// ---------------------------------------------------------------------------------------------

#[pyclass(module = "rpopt._rpopt", frozen, name = "CriticalRegion")]
pub struct CriticalRegion {
    inner: rpopt::CriticalRegion,
    #[pyo3(get, name = "A")]
    a: Py<PyArray2<f64>>,
    #[pyo3(get)]
    b: Py<PyArray2<f64>>,
    #[pyo3(get, name = "C")]
    c: Py<PyArray2<f64>>,
    #[pyo3(get)]
    d: Py<PyArray2<f64>>,
    #[pyo3(get, name = "E")]
    e: Py<PyArray2<f64>>,
    #[pyo3(get)]
    f: Py<PyArray2<f64>>,
    #[pyo3(get)]
    active_set: Vec<usize>,
    #[pyo3(get)]
    omega_set: Vec<usize>,
    #[pyo3(get)]
    lambda_set: Vec<usize>,
    #[pyo3(get)]
    regular_set: (Vec<usize>, Vec<usize>),
    #[pyo3(get)]
    y_fixation: Option<Py<PyArray2<f64>>>,
    #[pyo3(get)]
    y_indices: Option<Vec<usize>>,
    #[pyo3(get)]
    x_indices: Option<Vec<usize>>,
}

fn wrap_region(py: Python<'_>, r: &rpopt::CriticalRegion) -> CriticalRegion {
    CriticalRegion {
        inner: r.clone(),
        a: mat_to(py, &r.a).unbind(),
        b: col_to(py, &r.b).unbind(),
        c: mat_to(py, &r.c).unbind(),
        d: col_to(py, &r.d).unbind(),
        e: mat_to(py, &r.e).unbind(),
        f: col_to(py, &r.f).unbind(),
        active_set: r.active_set.clone(),
        omega_set: r.omega_set.clone(),
        lambda_set: r.lambda_set.clone(),
        regular_set: r.regular_set.clone(),
        y_fixation: r.y_fixation.as_ref().map(|y| col_to(py, y).unbind()),
        y_indices: r.y_indices.clone(),
        x_indices: r.x_indices.clone(),
    }
}

#[pymethods]
impl CriticalRegion {
    fn evaluate<'py>(&self, py: Python<'py>, theta: V<'_>) -> PyResult<Bound<'py, PyAny>> {
        let t = theta_from(&theta);
        check_dim(t.v.len(), self.inner.e.cols)?;
        Ok(out_like(py, &self.inner.evaluate(&t.v), t.column))
    }

    fn lagrange_multipliers<'py>(&self, py: Python<'py>, theta: V<'_>) -> PyResult<Bound<'py, PyAny>> {
        let t = theta_from(&theta);
        check_dim(t.v.len(), self.inner.e.cols)?;
        Ok(out_like(py, &self.inner.lagrange_multipliers(&t.v), t.column))
    }

    #[pyo3(signature = (theta, tol=1e-5))]
    fn is_inside(&self, theta: V<'_>, tol: f64) -> PyResult<bool> {
        let t = vec_from(&theta);
        check_dim(t.len(), self.inner.e.cols)?;
        Ok(self.inner.is_inside(&t, tol))
    }

    fn is_full_dimension(&self, py: Python<'_>) -> bool {
        let r = self.inner.clone();
        py.detach(|| r.is_full_dimension())
    }

    fn get_constraints<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        PyList::new(py, [self.e.bind(py).clone().into_any(), self.f.bind(py).clone().into_any()]).unwrap()
    }

    fn __repr__(&self) -> String {
        format!("Critical region with active set {:?}", self.inner.active_set)
    }
}

// ---------------------------------------------------------------------------------------------
// solutions
// ---------------------------------------------------------------------------------------------

#[pyclass(module = "rpopt._rpopt", name = "Solution")]
pub struct Solution {
    sol: Arc<rpopt::Solution>,
    program: Py<PyAny>,
    regions: Py<PyList>,
}

fn make_solution(py: Python<'_>, program: &Bound<'_, PyAny>, sol: rpopt::Solution) -> PyResult<Solution> {
    let items: Vec<Py<CriticalRegion>> =
        sol.critical_regions.iter().map(|r| Py::new(py, wrap_region(py, r))).collect::<PyResult<_>>()?;
    let regions = PyList::new(py, items)?.unbind();
    Ok(Solution { sol: Arc::new(sol), program: program.clone().unbind(), regions })
}

#[pymethods]
impl Solution {
    #[getter]
    fn program(&self, py: Python<'_>) -> Py<PyAny> {
        self.program.clone_ref(py)
    }
    #[getter]
    fn critical_regions(&self, py: Python<'_>) -> Py<PyList> {
        self.regions.clone_ref(py)
    }
    #[getter]
    fn is_overlapping(&self) -> bool {
        self.sol.is_overlapping
    }
    #[getter]
    fn point_location_tolerance(&self) -> f64 {
        self.sol.point_location_tolerance
    }

    fn theta_dim(&self) -> usize {
        self.sol.theta_dim()
    }

    fn __len__(&self) -> usize {
        self.sol.len()
    }

    /// Index of the region containing theta, or None.
    fn get_region_index(&self, theta: V<'_>) -> PyResult<Option<usize>> {
        let t = vec_from(&theta);
        check_dim(t.len(), self.sol.theta_dim())?;
        Ok(self.sol.get_region_index(&t))
    }

    fn get_region(&self, py: Python<'_>, theta: V<'_>) -> PyResult<Option<Py<PyAny>>> {
        Ok(self.get_region_index(theta)?.map(|i| self.regions.bind(py).get_item(i).unwrap().unbind()))
    }

    /// Optimal x at theta, or None outside the feasible set. Same layout as theta (column in, column out).
    fn evaluate<'py>(&self, py: Python<'py>, theta: V<'_>) -> PyResult<Option<Bound<'py, PyAny>>> {
        let t = theta_from(&theta);
        check_dim(t.v.len(), self.sol.theta_dim())?;
        Ok(self.sol.evaluate(&t.v).map(|x| out_like(py, &x, t.column)))
    }

    fn evaluate_objective(&self, theta: V<'_>) -> PyResult<Option<f64>> {
        let t = vec_from(&theta);
        check_dim(t.len(), self.sol.theta_dim())?;
        Ok(self.sol.evaluate_objective(&t))
    }

    /// Batch evaluation. Returns (X, found): X is (n, n_x), rows outside the feasible set are NaN.
    fn evaluate_many<'py>(
        &self,
        py: Python<'py>,
        thetas: PyReadonlyArray2<'py, f64>,
    ) -> PyResult<(Bound<'py, PyArray2<f64>>, Bound<'py, PyArray1<bool>>)> {
        let dim = self.sol.theta_dim();
        check_dim(thetas.shape()[1], dim)?;
        let data: Vec<f64> = thetas.as_array().iter().cloned().collect();
        let sol = self.sol.clone();
        let nx = sol.critical_regions.first().map(|r| r.a.rows).unwrap_or(0);
        let rows: Vec<Option<Vec<f64>>> = py.detach(|| data.par_chunks(dim.max(1)).map(|t| sol.evaluate(t)).collect());
        let n = rows.len();
        let mut flat = vec![f64::NAN; n * nx];
        let mut found = vec![false; n];
        for (i, r) in rows.into_iter().enumerate() {
            if let Some(x) = r {
                flat[i * nx..(i + 1) * nx].copy_from_slice(&x);
                found[i] = true;
            }
        }
        Ok((Array2::from_shape_vec((n, nx), flat).unwrap().into_pyarray(py), Array1::from(found).into_pyarray(py)))
    }

    fn verify_solution(&self, py: Python<'_>) -> bool {
        let s = self.sol.clone();
        py.detach(|| s.verify_solution())
    }

    fn verify_theta(&self, py: Python<'_>, theta: V<'_>) -> PyResult<bool> {
        let t = vec_from(&theta);
        check_dim(t.len(), self.sol.theta_dim())?;
        let s = self.sol.clone();
        Ok(py.detach(|| s.verify_theta(&t)))
    }

    fn __repr__(&self) -> String {
        format!("Solution({} critical regions)", self.sol.len())
    }
}

fn solution_of(obj: &Bound<'_, PyAny>) -> PyResult<Arc<rpopt::Solution>> {
    let b = obj.downcast::<Solution>().map_err(|_| PyValueError::new_err("expected a Solution"))?;
    Ok(b.borrow().sol.clone())
}

/// Algorithm names accepted by `solve_mpqp`.
#[pyfunction]
fn algorithm_names() -> Vec<&'static str> {
    MpqpAlgorithm::all().iter().map(|a| a.name()).collect()
}

#[pyfunction]
#[pyo3(signature = (program, algorithm, seed=0, graph_samples=100))]
fn solve(py: Python<'_>, program: &Bound<'_, PyAny>, algorithm: &str, seed: u64, graph_samples: usize) -> PyResult<Solution> {
    let p = program_of(program)?;
    let alg = MpqpAlgorithm::from_name(algorithm).ok_or_else(|| {
        PyValueError::new_err(format!("unknown algorithm {algorithm:?}; available: {:?}", algorithm_names()))
    })?;
    let opts = SolveOptions { seed, graph_samples };
    let sol = py
        .detach(|| std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rpopt::solve_mpqp_with(&p, alg, &opts))))
        .map_err(|_| PyRuntimeError::new_err("the Rust solver panicked (algorithm not applicable to this program?)"))?;
    make_solution(py, program, sol)
}

// ---------------------------------------------------------------------------------------------
// point location
// ---------------------------------------------------------------------------------------------

/// The Rust structure is kept alive by an Arc; the geometry is rebuilt per locator (it is cheap
/// next to the locator itself).
struct Loc {
    inner: Arc<dyn PointLocator>,
    dim: usize,
}

impl Loc {
    fn locate(&self, theta: &V<'_>) -> PyResult<i64> {
        let t = vec_from(theta);
        check_dim(t.len(), self.dim)?;
        Ok(self.inner.locate(&t).map(|i| i as i64).unwrap_or(-1))
    }

    fn locate_many<'py>(&self, py: Python<'py>, thetas: PyReadonlyArray2<'py, f64>, parallel: bool) -> PyResult<Bound<'py, PyArray1<i64>>> {
        check_dim(thetas.shape()[1], self.dim)?;
        let data: Vec<f64> = thetas.as_array().iter().cloned().collect();
        let inner = self.inner.clone();
        let dim = self.dim.max(1);
        let out: Vec<i64> = py.detach(|| {
            let f = |t: &[f64]| inner.locate(t).map(|i| i as i64).unwrap_or(-1);
            if parallel {
                data.par_chunks(dim).map(f).collect()
            } else {
                data.chunks(dim).map(f).collect()
            }
        });
        Ok(Array1::from(out).into_pyarray(py))
    }
}

macro_rules! locator_class {
    ($name:ident, $pyname:literal) => {
        #[pyclass(module = "rpopt._rpopt", frozen, name = $pyname)]
        pub struct $name {
            loc: Loc,
            info: Vec<(&'static str, f64)>,
        }

        #[pymethods]
        impl $name {
            /// Index of the critical region containing theta, -1 if none.
            fn locate(&self, theta: V<'_>) -> PyResult<i64> {
                self.loc.locate(&theta)
            }

            /// Locate every row of `thetas` (n, n_t) with the GIL released; returns int64 indices, -1 if none.
            #[pyo3(signature = (thetas, parallel=true))]
            fn locate_many<'py>(&self, py: Python<'py>, thetas: PyReadonlyArray2<'py, f64>, parallel: bool) -> PyResult<Bound<'py, PyArray1<i64>>> {
                self.loc.locate_many(py, thetas, parallel)
            }

            #[getter]
            fn name(&self) -> &'static str {
                self.loc.inner.name()
            }

            /// Number of stored floating point values (online memory footprint).
            fn storage_floats(&self) -> usize {
                self.loc.inner.storage_floats()
            }

            /// Construction statistics (depth, cell counts, ...).
            #[getter]
            fn info(&self) -> std::collections::HashMap<&'static str, f64> {
                self.info.iter().cloned().collect()
            }

            fn __repr__(&self) -> String {
                format!("{}({})", $pyname, self.loc.inner.name())
            }
        }
    };
}

locator_class!(SequentialSearch, "SequentialSearch");
locator_class!(BinarySearchTree, "BinarySearchTree");
locator_class!(MultiwayTree, "MultiwayTree");
locator_class!(LinearMachine, "LinearMachine");
locator_class!(DescriptorFunction, "DescriptorFunction");
locator_class!(HashTable, "HashTable");
locator_class!(Yann, "Yann");

fn mk(sol: &Arc<rpopt::Solution>, inner: Arc<dyn PointLocator>, info: Vec<(&'static str, f64)>) -> Loc {
    let _ = info;
    Loc { inner, dim: sol.theta_dim() }
}

fn build_bst_rows(geom: &pl::PartitionGeometry) -> Vec<f64> {
    let nt = geom.dim;
    let mut rows = vec![];
    for i in 0..geom.num_regions {
        let (e, f) = geom.region_rows(i);
        for k in 0..f.len() {
            rows.extend_from_slice(&e[k * nt..(k + 1) * nt]);
            rows.push(f[k]);
        }
    }
    rows
}

#[pymethods]
impl SequentialSearch {
    #[new]
    fn new(py: Python<'_>, solution: &Bound<'_, PyAny>) -> PyResult<Self> {
        let s = solution_of(solution)?;
        let l = py.detach(|| pl::FlatSequential::new(&s));
        Ok(SequentialSearch { loc: mk(&s, Arc::new(l), vec![]), info: vec![] })
    }
}

#[pymethods]
impl BinarySearchTree {
    #[new]
    fn new(py: Python<'_>, solution: &Bound<'_, PyAny>) -> PyResult<Self> {
        let s = solution_of(solution)?;
        let (l, depth, nodes) = py.detach(|| {
            let g = pl::PartitionGeometry::new(&s);
            let b = pl::BinarySearchTree::new(&g);
            let (d, n) = (b.depth, b.num_nodes());
            (b, d, n)
        });
        Ok(BinarySearchTree { loc: mk(&s, Arc::new(l), vec![]), info: vec![("depth", depth as f64), ("nodes", nodes as f64)] })
    }
}

#[pymethods]
impl MultiwayTree {
    /// `levels` binary levels are collapsed into each multiway node (order 2^levels - 1).
    #[new]
    #[pyo3(signature = (solution, levels=3))]
    fn new(py: Python<'_>, solution: &Bound<'_, PyAny>, levels: usize) -> PyResult<Self> {
        let s = solution_of(solution)?;
        let (l, h, o) = py.detach(|| {
            let g = pl::PartitionGeometry::new(&s);
            let b = pl::BinarySearchTree::new(&g);
            let m = pl::MultiwayTree::from_bst(&b, levels, build_bst_rows(&g), g.offsets.clone());
            let (h, o) = (m.height, m.order());
            (m, h, o)
        });
        Ok(MultiwayTree { loc: mk(&s, Arc::new(l), vec![]), info: vec![("height", h as f64), ("order", o as f64)] })
    }
}

#[pymethods]
impl LinearMachine {
    #[new]
    fn new(py: Python<'_>, solution: &Bound<'_, PyAny>) -> PyResult<Self> {
        let s = solution_of(solution)?;
        let l = py.detach(|| pl::LinearMachine::new(&pl::PartitionGeometry::new(&s)));
        let l = l.ok_or_else(|| PyRuntimeError::new_err("the linear machine LP is infeasible (the partition is not facet-to-facet)"))?;
        let info = vec![("certified", l.report.certified as u8 as f64), ("centers_correct", l.report.centers_correct as f64)];
        Ok(LinearMachine { loc: mk(&s, Arc::new(l), vec![]), info })
    }
}

#[pymethods]
impl DescriptorFunction {
    #[new]
    #[pyo3(signature = (solution, seed=7))]
    fn new(py: Python<'_>, solution: &Bound<'_, PyAny>, seed: u64) -> PyResult<Self> {
        let s = solution_of(solution)?;
        let (l, cf, er) = py.detach(|| {
            let g = pl::PartitionGeometry::new(&s);
            let d = pl::DescriptorFunction::new(&s, &g, seed);
            let (cf, er) = (d.compared_facets, d.explicit_rows);
            (d, cf, er)
        });
        Ok(DescriptorFunction { loc: mk(&s, Arc::new(l), vec![]), info: vec![("compared_facets", cf as f64), ("explicit_rows", er as f64)] })
    }
}

#[pymethods]
impl HashTable {
    /// `target_cells` defaults to 16 times the number of regions.
    #[new]
    #[pyo3(signature = (solution, target_cells=None))]
    fn new(py: Python<'_>, solution: &Bound<'_, PyAny>, target_cells: Option<usize>) -> PyResult<Self> {
        let s = solution_of(solution)?;
        let l = py.detach(|| {
            let g = pl::PartitionGeometry::new(&s);
            pl::HashTable::new(&g, target_cells.unwrap_or((g.num_regions * 16).max(4)))
        });
        let info = ht_info(&l);
        Ok(HashTable { loc: mk(&s, Arc::new(l), vec![]), info })
    }

    /// Hash table whose storage is at most `max_bytes` (the grid is chosen from predicted cost).
    #[staticmethod]
    fn with_storage_limit(py: Python<'_>, solution: &Bound<'_, PyAny>, max_bytes: usize) -> PyResult<Self> {
        let s = solution_of(solution)?;
        let l = py.detach(|| pl::HashTable::with_storage_limit(&pl::PartitionGeometry::new(&s), max_bytes));
        let info = ht_info(&l);
        Ok(HashTable { loc: mk(&s, Arc::new(l), vec![]), info })
    }

    fn storage_bytes(&self) -> usize {
        self.info.iter().find(|(k, _)| *k == "storage_bytes").map(|(_, v)| *v as usize).unwrap_or(0)
    }
}

fn ht_info(l: &pl::HashTable) -> Vec<(&'static str, f64)> {
    vec![
        ("cells", l.num_cells() as f64),
        ("storage_bytes", l.storage_bytes() as f64),
        ("mean_candidates", l.mean_candidates()),
        ("max_candidates", l.max_candidates() as f64),
    ]
}

#[pymethods]
impl Yann {
    #[new]
    fn new(py: Python<'_>, solution: &Bound<'_, PyAny>) -> PyResult<Self> {
        let s = solution_of(solution)?;
        let l = py.detach(|| pl::Yann::new(&s));
        Ok(Yann { loc: mk(&s, Arc::new(l), vec![]), info: vec![] })
    }
}

// ---------------------------------------------------------------------------------------------

#[pymodule]
fn _rpopt(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<MplpProgram>()?;
    m.add_class::<MpqpProgram>()?;
    m.add_class::<SolverOutput>()?;
    m.add_class::<CriticalRegion>()?;
    m.add_class::<Solution>()?;
    m.add_class::<SequentialSearch>()?;
    m.add_class::<BinarySearchTree>()?;
    m.add_class::<MultiwayTree>()?;
    m.add_class::<LinearMachine>()?;
    m.add_class::<DescriptorFunction>()?;
    m.add_class::<HashTable>()?;
    m.add_class::<Yann>()?;
    m.add_function(wrap_pyfunction!(solve, m)?)?;
    m.add_function(wrap_pyfunction!(algorithm_names, m)?)?;
    Ok(())
}
