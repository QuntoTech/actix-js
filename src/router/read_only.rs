use actix_web::http::Method;
use halfbrown::HashMap;
use lru::LruCache;
use matchit::{Params, Router};
use parking_lot::Mutex;
use std::num::NonZeroUsize;
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

#[derive(Clone)]
struct RouteCacheEntry {
  callback: &'static CallBackFunction,
  params: std::collections::HashMap<String, String>,
}

struct RouteCache {
  get_cache: Mutex<LruCache<String, RouteCacheEntry>>,
  post_cache: Mutex<LruCache<String, RouteCacheEntry>>,
  put_cache: Mutex<LruCache<String, RouteCacheEntry>>,
  patch_cache: Mutex<LruCache<String, RouteCacheEntry>>,
  delete_cache: Mutex<LruCache<String, RouteCacheEntry>>,
}

impl RouteCache {
  fn new(capacity: usize) -> Self {
    let cache_size = NonZeroUsize::new(capacity).unwrap();
    Self {
      get_cache: Mutex::new(LruCache::new(cache_size)),
      post_cache: Mutex::new(LruCache::new(cache_size)),
      put_cache: Mutex::new(LruCache::new(cache_size)),
      patch_cache: Mutex::new(LruCache::new(cache_size)),
      delete_cache: Mutex::new(LruCache::new(cache_size)),
    }
  }

  fn get_cache_for_method(
    &self,
    method: &Method,
  ) -> Option<&Mutex<LruCache<String, RouteCacheEntry>>> {
    match method {
      &Method::GET => Some(&self.get_cache),
      &Method::POST => Some(&self.post_cache),
      &Method::PUT => Some(&self.put_cache),
      &Method::PATCH => Some(&self.patch_cache),
      &Method::DELETE => Some(&self.delete_cache),
      _ => None,
    }
  }

  fn get(&self, route: &str, method: &Method) -> Option<RouteCacheEntry> {
    self
      .get_cache_for_method(method)?
      .lock()
      .get(route)
      .cloned()
  }

  fn put(&self, route: String, method: &Method, entry: RouteCacheEntry) {
    if let Some(cache_mutex) = self.get_cache_for_method(method) {
      cache_mutex.lock().put(route, entry);
    }
  }

  fn clear(&self) {
    self.get_cache.lock().clear();
    self.post_cache.lock().clear();
    self.put_cache.lock().clear();
    self.patch_cache.lock().clear();
    self.delete_cache.lock().clear();
  }
}

static ROUTE_CACHE: std::sync::OnceLock<RouteCache> = std::sync::OnceLock::new();

fn get_route_cache() -> &'static RouteCache {
  ROUTE_CACHE.get_or_init(|| RouteCache::new(1000))
}

// 🚀 LRU缓存优化的路由匹配函数 - 先查缓存，未命中再进行实际匹配
#[inline(always)]
pub fn get_route_with_params_cached(
  route: &str,
  method: Method,
) -> Option<(
  &'static CallBackFunction,
  std::collections::HashMap<String, String>,
)> {
  let cache = get_route_cache();

  // 🚀 第一步：尝试从缓存中获取
  if let Some(cached_entry) = cache.get(route, &method) {
    return Some((cached_entry.callback, cached_entry.params));
  }

  // 🚀 第二步：缓存未命中，进行实际路由匹配
  let checking = get_routers().get_for_actix_method(method.clone())?;
  let found = checking.at(route);

  match found {
    Ok(res) => {
      let std_params = params_to_std_map(&res.params);

      // 🚀 第三步：将匹配结果放入缓存（只缓存成功的匹配）
      let cache_entry = RouteCacheEntry {
        callback: res.value,
        params: std_params.clone(),
      };
      cache.put(route.to_string(), &method, cache_entry);

      Some((res.value, std_params))
    }
    Err(_) => None, // 失败的匹配不缓存，避免缓存污染
  }
}

// 🚀 清理路由缓存的公共函数 - 在路由更新时调用
pub fn clear_route_cache() {
  get_route_cache().clear();
}

// 🚀 修改现有函数使用缓存优化版本
#[inline(always)]
pub fn get_route_with_params(
  route: &str,
  method: Method,
) -> Option<(
  &'static CallBackFunction,
  std::collections::HashMap<String, String>,
)> {
  // 使用缓存优化版本
  get_route_with_params_cached(route, method)
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
