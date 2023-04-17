use rust_bert::pipelines::question_answering::{QuestionAnsweringModel, QaInput};
mod indexer;
use indexer::Indexer;
use pdf_extract;
use std::collections::HashSet;


fn runqa(qa_model: &QuestionAnsweringModel, context: &str, question: &str) {


    let question = String::from(question);
    let context = String::from(context);

    let answers = qa_model.predict(&[QaInput { question, context }], 3, 32);
    println!("{:?}", answers);
}

// write a function to parse a pdf and return a string
fn parse_pdf(filename: String) -> String {
    let out = pdf_extract::extract_text(filename).unwrap();
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let question = &args[1];
    println!("Question: {}", question);

    let mut indexer = indexer::IndexerImpl::new();

    let binding = parse_pdf(String::from("resume.pdf"));
    let context = binding.as_str();
    indexer.add_document(context, "my data");

    indexer.add_document("Paris is capital of France.", "cap1");
    indexer.add_document("Delhi is capital of India.", "cap2");
    let res = indexer.retrieve_document(question);
    let mut text = String::new();
    let mut sources = HashSet::new();
    for i in res {
        text.push_str(i.text.as_str());
        sources.insert(i.loc);
    }

    let qa_model = QuestionAnsweringModel::new(Default::default()).unwrap();
    let answer = runqa(&qa_model, &text, question);
    println!("answer: {:?} sources: {:?}", answer, sources);

    // wait until Ctrl-C is pressed
    std::thread::park();
}
