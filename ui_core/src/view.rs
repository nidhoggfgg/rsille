use std::io::{self, Stdout};

use terminal::crossterm::{cursor::MoveTo, queue, style::Print};

use crate::{panel::Panel, traits::Draw, Update};

pub struct View<T> {
    buffer: T,
    panel: Panel,
}

impl View<Stdout> {
    pub fn new(panel: Panel) -> View<Stdout> {
        View {
            buffer: io::stdout(),
            panel,
        }
    }
}

impl<T> View<T>
where
    T: io::Write + Sized,
{
    pub fn with_buffer(buffer: T, panel: Panel) -> View<T> {
        View { buffer, panel }
    }

    pub fn print(&mut self) -> io::Result<()> {
        // queue!(self.buffer, Clear(ClearType::All))?;
        queue!(self.buffer, MoveTo(0, 0))?;
        let data = self
            .panel
            .draw()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "draw error"))?;
        let (width, _) = self
            .panel
            .size()
            .ok_or(io::Error::new(io::ErrorKind::Other, "draw error"))?;

        for chunk in data.chunks(width as usize) {
            for v in chunk {
                v.queue(&mut self.buffer)?;
                // queue!(self.buffer, Print("a"))?;
            }
            queue!(self.buffer, Print("\n"))?;
        }

        self.buffer.flush()?;
        Ok(())
    }

    pub async fn run(&mut self) -> io::Result<()> {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(30));
        loop {
            interval.tick().await;
            let updated = self.panel.update().await.unwrap();
            if updated {
                self.print()?;
            }
        }
    }
}
