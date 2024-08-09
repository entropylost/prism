use std::{
    collections::HashMap,
    ops::{Deref, Index, IndexMut},
};

use nalgebra::{SVector as Vector, Vector2};
use rand::{Rng, RngCore, SeedableRng};
use rand_pcg::Pcg64Mcg;
use smallvec::SmallVec;

pub mod base;
pub mod ext;
pub mod shape;
pub mod solver;
pub mod utils;
use base::*;
pub use ext::{GridSettings, PackedSettings, ParticleSettings, Volume};
use solver::*;
use utils::*;
