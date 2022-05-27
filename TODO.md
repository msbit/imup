## TODO

* Track messages in a map between `message_id` and output path, and if found use [`hard_link`](https://doc.rust-lang.org/std/fs/fn.hard_link.html) to avoid downloading redundant data
* Consider using [`threadpool`](https://docs.rs/threadpool/latest/threadpool) with a worker per folder 
