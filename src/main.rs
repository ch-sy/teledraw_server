use std::net::{TcpListener, TcpStream};
use std::thread;
use std::sync::{Arc, Mutex};
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use std::thread::sleep;
use std::time::Duration;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::sync::mpsc::channel;

enum Event {
    DrawLine(f32, f32, f32, f32)
}

struct Socket {

}

fn client_output(mut stream_out: TcpStream, events: Receiver<Event>) {
    for event in events {
        match event {
            Event::DrawLine(x1, y1, x2, y2) => {
                stream_out.write_f32::<LittleEndian>(x1).unwrap();
                stream_out.write_f32::<LittleEndian>(y1).unwrap();
                stream_out.write_f32::<LittleEndian>(x2).unwrap();
                stream_out.write_f32::<LittleEndian>(y2).unwrap();
            }
        }
    }

}

fn client_input(mut stream_in: TcpStream, clients: Arc<Mutex<Vec<Sender<Event>>>>, lines: Arc<Mutex<Vec<(f32,f32,f32,f32)>>>) {
    loop {
        let old_pos_x = stream_in.read_f32::<LittleEndian>().unwrap();
        let old_pos_y = stream_in.read_f32::<LittleEndian>().unwrap();
        let new_pos_x = stream_in.read_f32::<LittleEndian>().unwrap();
        let new_pos_y = stream_in.read_f32::<LittleEndian>().unwrap();
        {
            // Aktuellisiere die Linien auf dem Server
            let mut lines = lines.lock().unwrap();
            lines.push((old_pos_x, old_pos_y, new_pos_x, new_pos_y));
        }
        {
            // Sende die Neuerungen an alle anderen clients
            let mut clients = clients.lock().unwrap();

            for client in clients.iter() {
                client.send(Event::DrawLine(old_pos_x, old_pos_y, new_pos_x, new_pos_y));
            }
        }
    }
}

fn handle_client(mut stream: TcpStream, clients: Arc<Mutex<Vec<Sender<Event>>>>, lines: Arc<Mutex<Vec<(f32,f32,f32,f32)>>>) {
    println!("Neue Verbindung!");
    {
        let mut lines = lines.lock().unwrap();
        stream.write_u32::<LittleEndian>(lines.len() as u32);
        for line in lines.iter() {
            stream.write_f32::<LittleEndian>(line.0).unwrap();
            stream.write_f32::<LittleEndian>(line.1).unwrap();
            stream.write_f32::<LittleEndian>(line.2).unwrap();
            stream.write_f32::<LittleEndian>(line.3).unwrap();
        }
    }
    let (sender, receiver) = channel();
    {
        let mut clients = clients.lock().unwrap();
        clients.push(sender);
    }
    let mut out_stream = stream.try_clone().unwrap();
    thread::spawn(|| {client_output(out_stream, receiver)});
    thread::spawn(|| {client_input(stream, clients, lines)});

}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:2020")?;
    let lines = Arc::new(Mutex::new(vec![] as Vec<(f32,f32,f32,f32)>));
    let clients = Arc::new(Mutex::new(vec![] as Vec<Sender<Event>>));

    // accept connections and process them serially
    for stream in listener.incoming() {

        match stream {
            Ok(stream) => {
                let c_lines = lines.clone();
                let c_clients = clients.clone();
                thread::spawn(|| {handle_client(stream, c_clients, c_lines)});
            }
            Err(e) => { /* connection failed */ }
        }
    }
    Ok(())
}