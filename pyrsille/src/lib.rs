mod pycanvas;

use pycanvas::Canvas;
use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
fn pyrsille(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // register_child_module(m)?;
    m.add_class::<Canvas>()?;
    Ok(())
}

// fn register_child_module(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
//     let child_module = PyModule::new(parent_module.py(), "canvas")?;
//     // child_module.add_function(wrap_pyfunction!(new, &child_module)?)?;
//     child_module.add_class::<Canvas>()?;
//     parent_module.add_submodule(&child_module)
// }
