use async_trait::async_trait;

use crate::{style::Stylized, DrawErr};

pub trait Draw {
    fn draw(&self) -> Result<Vec<Stylized>, DrawErr>;
    fn size(&self) -> Option<(u32, u32)>;
}

#[async_trait]
pub trait Update: Send {
    async fn update(&mut self) -> Result<bool, DrawErr>;
}

// this trait is for making trait object
#[async_trait]
pub trait DrawUpdate: Draw + Update {}
