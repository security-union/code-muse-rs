// TODO: Check OS
use anyhow::bail;
use async_openai::{
    types::{ChatCompletionRequestMessageArgs, CreateChatCompletionRequestArgs, Role},
    Client,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::Write,
    process::{Child, Command, Stdio},
};
use text_io::read;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Describe what the program should do, be as specific as possible
    #[arg(short, long)]
    description: String,

    /// Programming language to generate this project in
    #[arg(short, long, default_value = "rust")]
    language: String,

    /// The name of the project which will be the name of the directory created
    #[arg(short, long, default_value = "myapp")]
    name: String,
}

#[derive(Deserialize, Serialize)]
struct OutputJson {
    steps: Vec<String>,
    files: HashMap<String, String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let prompt = format!(
            "Take the following programming language, application requirements, and project name then generate two things.
            1. The steps to create the project in valid bash
            2. The files that will complete the application requirements that will actually compile and work

            The output must match the provided output json schema and be a valid json.

            Project Name:
            {name}

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
            name=args.name,
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
    println!("Sending prompt to OpenAI, please wait... 🤖");
    let res = client.chat().create(req).await?;
    println!("Got a response ✅ Attempting to decode the contents...");
    println!("Response:\n{}", &res.choices[0].message.content);
    let contents: OutputJson = serde_json::from_str(&res.choices[0].message.content)?;
    println!("Success, the robot has obeyed our orders.\n");

    println!("Let's setup the project");

    for step in contents.steps.iter() {
        println!("Does this step look correct? `{step}`");
        println!("Please input Y/N");
        let response: String = read!("{}\n");
        let cmd = {
            if response.replace(" ", "").to_uppercase() == "Y" {
                step.clone()
            } else {
                println!("Damn robot, can you fix the command? Otherwise enter `skip` to go to the next step");
                let response: String = read!("{}\n");
                if response.contains("skip") {
                    continue;
                } else {
                    response
                }
            }
        };

        let mut child = Command::new("bash")
            .arg("-c")
            .arg(&cmd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let status = child.wait()?;
        if !status.success() {
            bail!(
                "Failed to execute `{}`. Exit code: {:?}",
                cmd,
                status.code()
            );
        }
    }

    let project_root = std::path::Path::new(&args.name);
    if !project_root.exists() {
        println!();
    }

    for (file_path, file_contents) in contents.files {
        let path_str = format!("{}/{}", args.name, file_path);
        let path = std::path::Path::new(&path_str);
        if path.exists() {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(path)?;
            file.write_all(file_contents.as_bytes())?;
            file.flush()?;
        } else {
            println!("Unable to determine exactly where to put files. Please create this project file:\nFile path: {:?}\nFile Contents:\n{}", path, file_contents);
        }
    }

    Ok(())
}

