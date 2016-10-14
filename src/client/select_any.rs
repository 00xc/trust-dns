use std::mem;
use futures::{Future, IntoFuture, Poll, Async};

// TODO: drop this inner class once Futures.rs gets the final impl which can replace this.

/// Future for the `select_any` combinator, waiting for one of any of a list of
/// futures to succesfully complete. unlike `select_all`, this future ignores all
/// but the last error, if there are any.
///
/// This is created by this `select_any` function.
#[must_use = "futures do nothing unless polled"]
pub struct SelectAny<A> where A: Future {
    inner: Vec<A>,
}

/// Creates a new future which will select the first successful future over a list of futures.
///
/// The returned future will wait for any future within `list` to be ready and Ok. Unlike
/// select_all, this will only return the first successful completion, or the last
/// failure. This is useful in contexts where any success is desired and failures
/// are ignored, unless all the futures fail.
///
/// # Panics
///
/// This function will panic if the iterator specified contains no items.
pub fn select_any<I>(iter: I) -> SelectAny<<I::Item as IntoFuture>::Future>
    where I: IntoIterator,
          I::Item: IntoFuture,
{
  let ret = SelectAny {
    inner: iter.into_iter()
    .map(|a| a.into_future())
    .collect(),
  };
  assert!(ret.inner.len() > 0);
  ret
}

impl<A> Future for SelectAny<A> where A: Future {
  type Item = (A::Item, Vec<A>);
  type Error = A::Error;

  fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
    // loop until we've either exhausted all errors, a success was hit, or nothing is ready
    loop {
      let item = self.inner.iter_mut().enumerate().filter_map(|(i, f)| {
        match f.poll() {
          Ok(Async::NotReady) => None,
          Ok(Async::Ready(e)) => Some((i, Ok(e))),
          Err(e) => Some((i, Err(e))),
        }
      }).next();

      match item {
        Some((idx, res)) => {
          // always remove Ok or Err, if it's not the last Err continue looping
          drop(self.inner.remove(idx));
          match res {
            Ok(e) => {
              let rest = mem::replace(&mut self.inner, Vec::new());
              return Ok(Async::Ready((e, rest)))
            },
            Err(e) => {
              if self.inner.is_empty() {
                return Err(e)
              }
            },
          }
        }
        None => {
          // based on the filter above, nothing is ready, return
          return Ok(Async::NotReady)
        },
      }
    }
  }
}
