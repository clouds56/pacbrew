use crate::package::{mirror::MirrorServer, package::PkgBuild};


pub struct MirrorLists {
  lists: Vec<MirrorServer>,
}

pub enum ReqPath {
  Api(String),
  Package(PkgBuild),
}
