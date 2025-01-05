use std::io::{self, Stdout};

use crossterm::{cursor::MoveTo, queue, style::Print};

use crate::{panel::Panel, traits::Draw, Update};

pub struct Terminal<T> {
    buffer: T,
    panel: Panel,
}

impl Terminal<Stdout> {
    pub fn new(panel: Panel) -> Terminal<Stdout> {
        Terminal {
            buffer: io::stdout(),
            panel,
        }
    }
}

impl<T> Terminal<T>
where
    T: io::Write + Sized,
{
    pub fn with_buffer(buffer: T, panel: Panel) -> Terminal<T> {
        Terminal { buffer, panel }
    }

    pub fn print(&mut self) -> io::Result<()> {
        // queue!(self.buffer, Clear(ClearType::All))?;
        queue!(self.buffer, MoveTo(0, 0))?;
        let data = self.panel.draw();
        let (width, _) = self.panel.size();

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
