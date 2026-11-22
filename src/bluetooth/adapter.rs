use bluer::{Adapter, AdapterEvent, Error, Session};
use futures::StreamExt;
use std::collections::HashMap;

pub struct Adaptador {
    pub sesion: Session,
}

impl Adaptador {
    pub async fn new() -> Result<Self, Error> {
        let object = Session::new().await?;
        Ok(Adaptador { sesion: object })
    }

    pub async fn get_adapter(&self) -> Result<Adapter, Error> {
        let adapter = self.sesion.default_adapter().await?;
        Ok(adapter)
    }

    pub async fn start_discover(&self) -> Result<(), Error> {
        let adaptador = self.get_adapter().await?;
        let mut event = adaptador.discover_devices().await?;
        let mut map = HashMap::new();
        while let Some(events) = event.next().await {
            match events {
                AdapterEvent::DeviceAdded(address) => {
                    map.insert(address, adaptador.device(address)?);
                    println!("Dispositivo encontrado: {address}");
                }

                AdapterEvent::DeviceRemoved(address) => {
                    map.remove(&address);
                }

                AdapterEvent::PropertyChanged(_) => {}
            }
        }
        println!("Total de dispositivos: {}", map.len());
        Ok(())
    }
}
