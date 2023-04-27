use clap::Parser;
use async_openai::{Client, types::{CreateChatCompletionRequestArgs, ChatCompletionRequestMessageArgs, Role}};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Describe what the program should do, be as specific as possible
    #[arg(short, long)]
    description: String,

    /// Programming language to generate this project in
    #[arg(short, long, default_value="rust")]
    language: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let prompt = format!(
            "Take the following programming language and application requirements and respond with the bash steps to create the project and the files that will fulfill the application requirements.

            Programming Language:
            {language}

            Application Requirements:
            {description}

            Output Json schema:
            {{
                \"steps\": [\"step 1\", \"step 2\"],
                \"files\": {{
                    \"path/file1.ext\": \"contents\"
                }}
            }}

            Respond ONLY with the data portion of a valid Json object. No schema definition required. No other words.",
            language=args.language,
            description=args.description
    );
    let client = Client::new();
    let req = CreateChatCompletionRequestArgs::default().max_tokens(2048u16).model("gpt-3.5-turbo").messages([
        ChatCompletionRequestMessageArgs::default().role(Role::System).content("You are a helpful programming assistant.
You are expected to process an application description and generate the files and steps necessary to create the application as using your language model.
You can only respond with a Json object that matches the provided output schema.
The return Json can include an array of objects as defined by the output schema.
You are not allowed to return anything but a valid Json object.").build()?,
        ChatCompletionRequestMessageArgs::default().role(Role::User).content(prompt).build()?
    ]).build()?;
    let res = client.chat().create(req).await?;
    println!("\nResponse:\n");
    for choice in res.choices {
        println!(
            "{}: Role: {}  Content: {}",
            choice.index, choice.message.role, choice.message.content
        );
    }

    Ok(())
}

