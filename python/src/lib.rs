//! Fast path for float/int/bool data. Returns NotImplemented-style None when the data needs the stdlib (Fraction, Decimal, subclasses, huge ints).
use num_bigint::BigInt;
use pyo3::exceptions::{PyOverflowError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyFloat, PyInt, PyList, PyTuple};
use statistics_core::{convert, mean_frac, sqrt_of_frac, ss_over, sums, Num, Out, Overflow, Sums};

fn push(out: &mut Vec<Num>, x: &Bound<'_, PyAny>) -> PyResult<bool> {
    if let Ok(f) = x.cast_exact::<PyFloat>() {
        out.push(Num::F(f.value()));
    } else if x.is_exact_instance_of::<PyBool>() {
        out.push(Num::I(x.is_truthy()? as i64));
    } else if x.is_exact_instance_of::<PyInt>() {
        match x.extract::<i64>() {
            Ok(v) => out.push(Num::I(v)),
            Err(_) => return Ok(false),
        }
    } else {
        return Ok(false);
    }
    Ok(true)
}

fn collect(seq: &Bound<'_, PyAny>) -> PyResult<Option<Vec<Num>>> {
    let mut out = Vec::new();
    if let Ok(l) = seq.cast::<PyList>() {
        out.reserve(l.len());
        for x in l.iter() {
            if !push(&mut out, &x)? { return Ok(None); }
        }
    } else if let Ok(t) = seq.cast::<PyTuple>() {
        out.reserve(t.len());
        for x in t.iter() {
            if !push(&mut out, &x)? { return Ok(None); }
        }
    } else {
        return Ok(None);
    }
    Ok(Some(out))
}

fn stats_error(py: Python<'_>, msg: &str) -> PyErr {
    match py.import("statistics").and_then(|m| m.getattr("StatisticsError")).and_then(|c| c.call1((msg,))) {
        Ok(e) => PyErr::from_value(e),
        Err(e) => e,
    }
}

fn overflow(_: Overflow) -> PyErr {
    PyOverflowError::new_err("integer division result too large for a float")
}

fn out_obj(py: Python<'_>, o: Out) -> PyResult<Py<PyAny>> {
    Ok(match o {
        Out::Int(i) => i.into_pyobject(py)?.into_any().unbind(),
        Out::Float(f) => f.into_pyobject(py)?.into_any().unbind(),
    })
}

fn prepare(py: Python<'_>, data: &Bound<'_, PyAny>, squares: bool) -> PyResult<Option<Sums>> {
    let v = match collect(data)? {
        Some(v) => v,
        None => return Ok(None),
    };
    Ok(Some(py.detach(|| sums(&v, squares))))
}

/// mean(data) for float/int data, or None to let the stdlib handle it.
#[pyfunction]
fn mean(py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Option<Py<PyAny>>> {
    let Some(s) = prepare(py, data, false)? else { return Ok(None) };
    if s.n < 1 {
        return Err(stats_error(py, "mean requires at least one data point"));
    }
    if let Some(sp) = s.special {
        return Ok(Some((sp / s.n as f64).into_pyobject(py)?.into_any().unbind()));
    }
    let (n, d) = mean_frac(&s);
    Ok(Some(out_obj(py, convert(&n, &d, !s.any_float).map_err(overflow)?)?))
}

fn var_impl(py: Python<'_>, data: &Bound<'_, PyAny>, sample: bool, root: bool) -> PyResult<Option<Py<PyAny>>> {
    let Some(s) = prepare(py, data, true)? else { return Ok(None) };
    let name = match (sample, root) {
        (true, false) => "variance requires at least two data points",
        (false, false) => "pvariance requires at least one data point",
        (true, true) => "stdev requires at least two data points",
        (false, true) => "pstdev requires at least one data point",
    };
    if (sample && s.n < 2) || s.n < 1 {
        return Err(stats_error(py, name));
    }
    let divisor = if sample { s.n - 1 } else { s.n };
    if let Some(sp) = s.special {
        let v = sp / divisor as f64;
        if root {
            return Err(PyValueError::new_err("inf or nan encountered in data"));
        }
        return Ok(Some(v.into_pyobject(py)?.into_any().unbind()));
    }
    let (num, den) = ss_over(&s, divisor);
    if root {
        let r = py.detach(|| sqrt_of_frac(&num, &den)).map_err(overflow)?;
        return Ok(Some(r.into_pyobject(py)?.into_any().unbind()));
    }
    Ok(Some(out_obj(py, convert(&num, &den, !s.any_float).map_err(overflow)?)?))
}

#[pyfunction]
fn variance(py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Option<Py<PyAny>>> {
    var_impl(py, data, true, false)
}
#[pyfunction]
fn pvariance(py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Option<Py<PyAny>>> {
    var_impl(py, data, false, false)
}
#[pyfunction]
fn stdev(py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Option<Py<PyAny>>> {
    var_impl(py, data, true, true)
}
#[pyfunction]
fn pstdev(py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Option<Py<PyAny>>> {
    var_impl(py, data, false, true)
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let _ = BigInt::from(0);
    m.add_function(wrap_pyfunction!(mean, m)?)?;
    m.add_function(wrap_pyfunction!(variance, m)?)?;
    m.add_function(wrap_pyfunction!(pvariance, m)?)?;
    m.add_function(wrap_pyfunction!(stdev, m)?)?;
    m.add_function(wrap_pyfunction!(pstdev, m)?)?;
    Ok(())
}
