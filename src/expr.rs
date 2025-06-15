use std::cell::UnsafeCell;
use std::ffi::{CStr, CString};
use std::fmt;
use std::hash::{Hash, Hasher};

use symengine_sys::*;
use crate::{SymEngineError, SymEngineResult};

#[cfg(feature = "serde-serialize")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub struct Expression {
    pub(crate) basic: UnsafeCell<basic_struct>,
}

impl Expression {
    /// Create a new expression from a string representation
    pub fn new<T>(expr: T) -> Self
    where
        T: Into<Vec<u8>> + fmt::Display,
    {
        let expr_string = expr.to_string();
        let expr = CString::new(expr_string).expect("Failed to create CString");
        unsafe {
            let mut new = Expression {
                basic: UnsafeCell::new(std::mem::zeroed()),
            };
            basic_new_stack(new.basic.get());
            let result = basic_parse(new.basic.get(), expr.as_ptr());
            if result != 0 {
                // If parsing failed, create a symbol instead
                eprintln!("Warning: Failed to parse '{}', treating as symbol", expr.to_string_lossy());
            }
            new
        }
    }

    /// Create a new expression from a string, returning a Result
    pub fn try_new<T>(expr: T) -> SymEngineResult<Self>
    where
        T: Into<Vec<u8>> + fmt::Display,
    {
        let expr_string = expr.to_string();
        let expr = CString::new(expr_string).map_err(|_| {
            SymEngineError::invalid_operation("String contains null bytes")
        })?;
        
        unsafe {
            let mut new = Expression {
                basic: UnsafeCell::new(std::mem::zeroed()),
            };
            basic_new_stack(new.basic.get());
            let result = basic_parse(new.basic.get(), expr.as_ptr());
            if result != 0 {
                return Err(SymEngineError::ParseError);
            }
            Ok(new)
        }
    }

    /// Create a symbolic variable
    pub fn symbol<T>(name: T) -> Self
    where
        T: Into<Vec<u8>> + fmt::Display,
    {
        let name_string = name.to_string();
        let name_cstr = CString::new(name_string).expect("Failed to create CString for symbol name");
        unsafe {
            let mut new = Expression {
                basic: UnsafeCell::new(std::mem::zeroed()),
            };
            basic_new_stack(new.basic.get());
            symbol_set(new.basic.get(), name_cstr.as_ptr());
            new
        }
    }

    pub fn from_value<T, F>(f: F, value: T) -> Self
    where
        F: Fn(*mut basic_struct, T) -> c_int,
    {
        unsafe {
            let mut basic: basic_struct = std::mem::zeroed();
            basic_new_stack(&mut basic);
            f(&mut basic, value);
            let new = Expression {
                basic: UnsafeCell::new(basic),
            };
            new
        }
    }

    pub fn from_i64(value: i64) -> Self {
        Self::from_value(integer_set_si, value)
    }

    pub fn from_i32(value: i32) -> Self {
        Self::from_value(integer_set_si, value as i64)
    }

    pub fn from_u32(value: u32) -> Self {
        Self::from_value(integer_set_ui, value as c_ulong)
    }

    pub fn from_f64(value: f64) -> Self {
        Self::from_value(real_double_set_d, value)
    }

    pub fn from_f32(value: f32) -> Self {
        Self::from_value(real_double_set_d, value as f64)
    }

    pub fn clone(&self) -> Self {
        let mut new = Expression {
            basic: UnsafeCell::new(unsafe { std::mem::zeroed() }),
        };
        unsafe { basic_assign(new.basic.get(), self.basic.get()) };
        new
    }
}

impl Drop for Expression {
    fn drop(&mut self) {
        unsafe {
            basic_free_stack(self.basic.get());
        }
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let expr = unsafe { CStr::from_ptr(basic_str(self.basic.get())) };
        write!(f, "{}", expr.to_string_lossy())
    }
}

impl<T: Into<Self>> std::ops::Add<T> for Expression {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        self.binary_op(rhs.into(), basic_add)
    }
}

impl<T: Into<Self>> std::ops::Sub<T> for Expression {
    type Output = Self;

    fn sub(self, rhs: T) -> Self::Output {
        self.binary_op(rhs.into(), basic_sub)
    }
}

impl<T: Into<Self>> std::ops::Mul<T> for Expression {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        self.binary_op(rhs.into(), basic_mul)
    }
}

impl<T: Into<Self>> std::ops::Div<T> for Expression {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        self.binary_op(rhs.into(), basic_div)
    }
}

impl From<i32> for Expression {
    fn from(i: i32) -> Self {
        Self::from_i32(i)
    }
}

impl From<i64> for Expression {
    fn from(i: i64) -> Self {
        Self::from_i64(i)
    }
}

impl From<u32> for Expression {
    fn from(i: u32) -> Self {
        Self::from_u32(i)
    }
}

impl From<f64> for Expression {
    fn from(f: f64) -> Self {
        Self::from_f64(f)
    }
}

impl From<f32> for Expression {
    fn from(f: f32) -> Self {
        Self::from_f32(f)
    }
}

impl Expression {
    fn binary_op<F>(self, other: Self, op: F) -> Self
    where
        F: Fn(*mut basic_struct, *mut basic_struct, *mut basic_struct) -> c_int,
    {
        unsafe {
            let mut new = Expression {
                basic: UnsafeCell::new(std::mem::zeroed()),
            };
            basic_new_stack(new.basic.get());
            op(new.basic.get(), self.basic.get(), other.basic.get());
            new
        }
    }

    pub fn expand(&self) -> Self {
        unsafe {
            let mut new = Expression {
                basic: UnsafeCell::new(std::mem::zeroed()),
            };
            basic_new_stack(new.basic.get());
            basic_expand(new.basic.get(), self.basic.get());
            new
        }
    }

    fn cmp_eq_op<F>(&self, other: &Self, op: F) -> bool
    where
        F: Fn(*mut basic_struct, *mut basic_struct) -> i32,
    {
        let lhs = self.expand();
        let rhs = other.expand();
        let result = unsafe { op(lhs.basic.get(), rhs.basic.get()) };
        result != 0
    }
}

impl PartialEq for Expression {
    fn eq(&self, other: &Self) -> bool {
        self.cmp_eq_op(other, basic_eq)
    }
}

#[cfg(feature = "serde-serialize")]
impl Serialize for Expression {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = self.to_string();
        s.serialize(serializer)
    }
}

#[cfg(feature = "serde-serialize")]
impl<'de> Deserialize<'de> for Expression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Expression::new(s))
    }
}