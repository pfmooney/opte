//! Safe abstractions for synchronization primitives.
//!
//! TODO: This should be in its own crate, wrapping the illumos-ddi-dki
//! crate. But for now just let it live here.
#[cfg(all(not(feature = "std"), not(test)))]
extern crate core as std;

use illumos_ddi_dki::kmutex_type_t;
#[cfg(all(not(feature = "std"), not(test)))]
use illumos_ddi_dki::{kmutex_t, mutex_enter, mutex_exit, mutex_init};

#[cfg(all(not(feature = "std"), not(test)))]
use std::cell::UnsafeCell;
#[cfg(all(not(feature = "std"), not(test)))]
use std::convert::Infallible;
use std::ops::Deref;
#[cfg(all(not(feature = "std"), not(test)))]
use std::ops::DerefMut;
#[cfg(all(not(feature = "std"), not(test)))]
use std::ptr;

#[cfg(any(feature = "std", test))]
use std::sync::Mutex;

/// Exposes the illumos mutex(9F) API in a safe manner. We name it
/// `KMutex` (Kernel Mutex) on purpose. The API for a kernel mutex
/// isn't quite the same as a userland `Mutex`, and there's no reason
/// that we have to use that exact name. Using `KMutex` makes it
/// obvious that we are using a mutex, but not the one that comes from
/// std.
///
/// Our `kmutex_t` implementation has no self referential pointers, so
/// there is no reason it needs to pin to a location. This allows us to
/// have an API that looks more Rust-like: returning a `KMutex` value
/// that is initialized and can be placed anywhere. This is in contrast
/// to the typical illumos API where you have a `kmutex_t` embedded in
/// your structure (or as a global) and pass a pointer to
/// `mutex_init(9F)` to initialize it in place.
///
/// For now we assume only `Sized` types are protected by a mutex. For
/// some reason, rust-for-linux adds a `?Sized` bound for the type
/// definition as well as various impl blocks, minus the one that deals
/// with creating a new mutex. I'm not sure why they do this,
/// esepcially if the impl prevents you from creating a mutex holding a
/// DST. I'm not sure if a mutex should ever hold a DST, because a DST
/// is necessairly a pointer, and we would need to make sure that if a
/// shared reference was passed in that it's the only one outstanindg.
///
/// It seems the std Mutex also does this, but once against I'm not
/// sure why.
#[cfg(all(not(feature = "std"), not(test)))]
pub struct KMutex<T> {
    // The mutex(9F) structure.
    mutex: UnsafeCell<kmutex_t>,

    // I'm not sure if an illumos kernel mutex needs to be pinned, but
    // I've never seen any C code that "moves" a mutex. For now, we
    // pin it. The `PhantomPinned` marker type precludes the auto
    // implementation of the Unpin trait. That means that if you put
    // KMutex inside a Pin, you can't get mutable access to the
    // contents without going through an unsafe API like
    // `get_unchecked_mut()`.
    // _pin: PhantomPinned,

    // The data this mutex protects.
    data: UnsafeCell<T>,
}

pub enum KMutexType {
    Adaptive = kmutex_type_t::MUTEX_ADAPTIVE as isize,
    Spin = kmutex_type_t::MUTEX_SPIN as isize,
    Driver = kmutex_type_t::MUTEX_DRIVER as isize,
    Default = kmutex_type_t::MUTEX_DEFAULT as isize,
}

impl From<KMutexType> for kmutex_type_t {
    fn from(mtype: KMutexType) -> Self {
        match mtype {
            KMutexType::Adaptive => kmutex_type_t::MUTEX_ADAPTIVE,
            KMutexType::Spin => kmutex_type_t::MUTEX_SPIN,
            KMutexType::Driver => kmutex_type_t::MUTEX_DRIVER,
            KMutexType::Default => kmutex_type_t::MUTEX_DEFAULT,
        }
    }
}

// TODO understand:
//
// o Why does rust-for-linux use `T: ?Sized` for struct def.
//
// o Why is the guard referred to as an RAII guard.
#[cfg(all(not(feature = "std"), not(test)))]
impl<T> KMutex<T> {
    /// Create, initialize, and return a new kernel mutex (mutex(9F))
    /// of type `mtype`, and wrap it around `val`. The returned
    /// `KMutex` is the new owner of `val`. All access from here on out
    /// must be done by acquiring a `KMutexGuard` via the `lock()`
    /// method.
    pub fn new(val: T, mtype: KMutexType) -> Self {
        let mut kmutex = kmutex_t { _opaque: 0 };
        // TODO This assumes the mutex is never used in interrupt
        // context. Need to pass 4th arg to set priority.
        //
        // We never use the mutex name argument.
        //
        // Safety: ???.
        unsafe {
            mutex_init(&mut kmutex, ptr::null(), mtype.into(), ptr::null());
        }

        KMutex { mutex: UnsafeCell::new(kmutex), data: UnsafeCell::new(val) }
    }

    /// Try to acquire the mutex guard to gain access to the underlying
    /// value. If the guard is currently held, then this call will
    /// block. The mutex is released when the guard is dropped.
    ///
    /// We return a `Result` to be consistent with `Mutex` so that we
    /// can run opte-core tests in userland. However, `KMutex` will
    /// always return `Ok(...)`.
    pub fn lock(&self) -> Result<KMutexGuard<T>, Infallible> {
        // Safety: ???.
        unsafe { mutex_enter(self.mutex.get()) };
        Ok(KMutexGuard { lock: self })
    }
}

#[cfg(all(not(feature = "std"), not(test)))]
pub struct KMutexGuard<'a, T: 'a> {
    lock: &'a KMutex<T>,
}

#[cfg(all(not(feature = "std"), not(test)))]
impl<T> Drop for KMutexGuard<'_, T> {
    fn drop(&mut self) {
        // Safety: ???.
        unsafe { mutex_exit(self.lock.mutex.get()) };
    }
}

#[cfg(all(not(feature = "std"), not(test)))]
impl<T> Deref for KMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

#[cfg(all(not(feature = "std"), not(test)))]
impl<T> DerefMut for KMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

// In a std environment we just wrap `Mutex`.
#[cfg(any(feature = "std", test))]
pub struct KMutex<T> {
    inner: Mutex<T>,
}

#[cfg(any(feature = "std", test))]
impl<T> KMutex<T> {
    pub fn new(val: T, _mtype: KMutexType) -> Self {
        KMutex { inner: Mutex::new(val) }
    }
}

#[cfg(any(feature = "std", test))]
impl<T> Deref for KMutex<T> {
    type Target = Mutex<T>;

    fn deref(&self) -> &Mutex<T> {
        &self.inner
    }
}