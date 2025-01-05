use async_trait::async_trait;

use crate::style::Stylized;

#[derive(Debug, Clone, Copy)]
pub struct DrawErr;

pub trait Draw {
    fn draw(&self) -> Vec<Stylized>;
    fn size(&self) -> (u32, u32);
}

#[async_trait]
pub trait Update: Send {
    async fn update(&mut self) -> Result<bool, DrawErr>;
}

#[async_trait]
pub trait DrawUpdate: Draw + Update {}
