use clap::Args;
use std::fs::read_to_string;
use std::{
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread::JoinHandle,
};
use urlencoding::decode;

#[derive(Debug, Args)]
pub struct ServeArgs {
    #[arg(short, long, required = false)]
    name: String,
    #[arg(short, long, required = false)]
    path: PathBuf,
}

impl ServeArgs {
    pub async fn handle_args(&self) -> i16 {
        let url = "127.0.0.1:7878";
        println!("Starting at http://{url}/{}", self.path.to_str().unwrap());
        serve(url).await;
        0
    }
}

struct Worker {
    id: usize,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Job>>>) -> Self {
        let thread = std::thread::Builder::new()
            .name(format!("{}", id))
            .spawn(move || loop {
                let message = receiver.lock().unwrap().recv();

                match message {
                    Ok(job) => {
                        job();
                    },
                    Err(_) => {
                        println!("Worker {id} disconnected; shutting down.");
                        break;
                    },
                }
            })
            .ok();
        Self { id, thread }
    }
}

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<Sender<Job>>,
}

impl ThreadPool {
    fn new(size: usize) -> Self {
        let (sender, receiver) = channel();

        let receiver = Arc::new(Mutex::new(receiver));

        Self {
            workers: (0..size)
                .map(|i| Worker::new(i, Arc::clone(&receiver)))
                .collect(),
            sender: Some(sender),
        }
    }
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

async fn serve(url: &str) {
    let listener = TcpListener::bind(url).unwrap();
    let pool = ThreadPool::new(4);
    listener
        .incoming()
        .for_each(|s| pool.execute(|| handle_connection(s.unwrap())));
}

fn good_path(markdown_input: &str) -> (String, String) {
    let parser = pulldown_cmark::Parser::new(markdown_input);

    // Write to a new String buffer.
    let mut contents = String::new();
    pulldown_cmark::html::push_html(&mut contents, parser);
    ("HTTP/1.1 200 OK".to_string(), contents)
}

fn handle_connection(mut stream: TcpStream) {
    let mut reader = BufReader::new(&mut stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).unwrap();
    let path = decode(
        request_line
            .split_whitespace()
            .collect::<Vec<_>>()
            .as_slice()
            .get(1)
            .unwrap()
            .to_owned()
            .get(1..)
            .unwrap(),
    )
    .unwrap();

    println!("{:?}", &path);
    let (status_line, contents) = match read_to_string(path.to_string()) {
        Ok(v) => good_path(&v),
        Err(err) => (
            "HTTP/1.1 404 NOT FOUND".to_string(),
            format!("<p>{}</p>", err),
        ),
    };
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).unwrap();
}
