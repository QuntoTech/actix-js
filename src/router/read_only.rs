use actix_web::http::Method;
use halfbrown::HashMap;
use matchit::{Params, Router};
use std::{cell::UnsafeCell, mem::MaybeUninit};

use crate::router::node_functions::CallBackFunction;

struct RouteCell(UnsafeCell<MaybeUninit<ReadRoutes>>);

unsafe impl Sync for RouteCell where ReadRoutes: Sync {}

type ReaderLookup = Router<CallBackFunction>;
static ROUTER: RouteCell = RouteCell(UnsafeCell::new(MaybeUninit::uninit()));

pub struct ReadRoutes {
  pub get: ReaderLookup,
  pub post: ReaderLookup,
  pub put: ReaderLookup,
  pub patch: ReaderLookup,
  pub delete: ReaderLookup,
}

impl ReadRoutes {
  #[inline(always)]
  fn get_for_actix_method(&self, method: Method) -> Option<&ReaderLookup> {
    match method {
      Method::GET => Some(&self.get),
      Method::POST => Some(&self.post),
      Method::PUT => Some(&self.put),
      Method::PATCH => Some(&self.patch),
      Method::DELETE => Some(&self.delete),
      _ => None,
    }
  }
}

pub fn write_reader(new_reader: ReadRoutes) {
  let router_ref = unsafe { &mut *ROUTER.0.get() };
  *router_ref = MaybeUninit::new(new_reader);
}

#[inline(always)]
fn get_routers() -> &'static ReadRoutes {
  unsafe { &*(*ROUTER.0.get()).as_ptr() }
}

#[inline(always)]
pub fn get_route_with_params(
  route: &str,
  method: Method,
) -> Option<(
  &'static CallBackFunction,
  std::collections::HashMap<String, String>,
)> {
  let checking = get_routers().get_for_actix_method(method)?;
  let found = checking.at(route);

  match found {
    Ok(res) => {
      let std_params = params_to_std_map(&res.params);
      Some((res.value, std_params))
    }
    Err(_) => None,
  }
}

#[inline(always)]
pub fn get_route(route: &str, method: Method) -> Option<&'static CallBackFunction> {
  let checking = get_routers().get_for_actix_method(method)?;
  let found = checking.at(route);

  match found {
    Ok(res) => Some(res.value),
    Err(_) => None,
  }
}

#[inline(always)]
fn params_to_map(params: &Params) -> HashMap<String, String> {
  let mut map = HashMap::with_capacity(params.len());

  for (key, value) in params.iter() {
    map.insert(key.to_string(), value.to_string());
  }

  map
}

#[inline(always)]
fn params_to_std_map(params: &Params) -> std::collections::HashMap<String, String> {
  let mut map = std::collections::HashMap::with_capacity(params.len());

  for (key, value) in params.iter() {
    map.insert(key.to_string(), value.to_string());
  }

  map
}

#[inline]
pub fn get_params(route: &str, method: Method) -> Option<HashMap<String, String>> {
  let checking = get_routers().get_for_actix_method(method)?;
  let found = checking.at(route);

  match found {
    Ok(res) => Some(params_to_map(&res.params)),
    Err(_) => None,
  }
}
