use std::fmt::write;

use glommio_play::{client, Result};
use glommio::{LocalExecutorBuilder};
use glommio::{io::DmaFile, ByteSliceMutExt, LocalExecutor};
async fn hello() {
    println!("Hello, world!");
}

pub fn main(){




    let builder = LocalExecutorBuilder::new().name("hello");
    let handler = builder.spawn(||  async move  {
        /*
        let mut dev_urandom = DmaFile::create("test").await.unwrap();
        let mut write_buf = dev_urandom.alloc_dma_buffer(1024);
        write_buf.write_at(0, "hello");
        dev_urandom.write_at(write_buf, 0).await.unwrap();
        hello().await;
        dev_urandom.close();
        */

        let mut client = client::connect("127.0.0.1:6379").await;

        client.set("hello", "world".into()).await.unwrap();
        client.set("hello", "world".into()).await.unwrap();
        client.set("hello", "world".into()).await.unwrap();
        client.set("hello", "world".into()).await.unwrap();

        
        // Set the key "hello" with value "world"
        let result = client.get("hello").await.unwrap();
        if let Some(value) = result {
            println!("{:?}", value);
        } else {
            println!("Key not found");
        }
    
    }).unwrap();

    handler.join().unwrap();
}
