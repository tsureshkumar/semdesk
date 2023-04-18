# SemDesk

SemDesk is a desktop tool and service to search files semantically. Instead of
matching files on names or keywords in the file, this tool tries to find the
answers from the contents of the file.

For instance, if you have saved a file deposits.txt with some content like "I
have $5000.", you can ask "How much do I have in my bank?".  This tool will
answer that "$5000", where as non-semantic search will return if there is a
keyword match in the file name.

Currently, this tool works with text files and some level of functionality with 
PDF files. PDF files are hard to parse for text as the flow may not be linear.

This uses facebook's faiss vector index for document retrieval and google's bert
model for question-answering.

# Building

Currently, the tool works on MacOS only.

You need to have rust installed and at least 5 GB disk space for downloading the
models, which is larger size.

```zsh
$ cargo build
```

# Running

The tool has two binaries. `semdesk` is background daemon that crawls the
configured directories once a day. It also has the backend for document
retrieval and answering queries. `semdesk-cli` contacts the daemon and executes
the query.

You also need pdf2ps and ps2ascii to extract text out of pdf files. You can
install these using `$ brew install ghostscript` on MacOS.

```zsh
$ cargo run --bin semdesk # run in a background terminal
$ semdesk-cli query "How much do I have in my bank?"
File: /Users/$USER/personal_docs/deposit_details.txt
Match Probability: 95.56
Rs.5000
```

# Configuration
The configuration for the daemon lives in `~/.config/semdesk/config.toml`
```toml
# ~/.config/semdesk/config.toml
[crawler]
files = [
    "~/Downloads/newsgroups/",
    "~/Downloads/personal_docs/",
    ]
max_depth = 3
```

# Other files
This writes the status of scanned files to `~/.local/share/semdesk*`.

# Details

This project uses [faiss](https://github.com/facebookresearch/faiss) for storing
the vector index. And it uses rust port of hugging face's transformers library
and bert model for question answering pipeline
[rust_bert](https://docs.rs/rust-bert/latest/rust_bert/).

I am able to run the indexer and model on my 4 year old macbook pro (16 GB RAM)
comfortably with a scan of about 1000+ small text files. Thanks to rust, it
takes only about 600 MB of RAM. The initial loading and during crawling, the
usage goes upto about 1.5 GB of RAM but the inference only takes 500+ MB. 

This is still in early stages of development, there could be some unknown
issues.

# License

This is free and open source software. You can use, copy, modify,
merge, publish, distribute, sublicense, and/or sell copies of it,
under the terms of the Apache 2.0 License. See [LICENSE.md][L] for details.

This software is provided "AS IS", WITHOUT WARRANTY OF ANY KIND,
express or implied. See [LICENSE.md][L] for details.

[L]: LICENSE.md

# Channels

The author of this project hangs out at the following places online:

- Twitter: [@tsureshkumar](https://twitter.com/tsureshkumar)
- Mastodon:
  [@tsureshkumar@mastodon.social](https://mastodon.social/@tsureshkumar)
- GitHub: [@tsureshkumar](https://github.com/tsureshkumar)

You are welcome to subscribe to, follow, or join one or more of the
above channels to receive updates from the author or ask questions
about this project.



