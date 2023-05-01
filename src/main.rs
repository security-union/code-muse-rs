use anyhow::bail;
use async_openai::{
    types::{ChatCompletionRequestMessageArgs, CreateChatCompletionRequestArgs, Role},
    Client,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{
    env::consts::OS,
    process::{Command, Stdio},
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
    script: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let prompt = format!(
            "Take the following programming language, application requirements, and project name then generate a valid bash script that does the following.
            1. Installs all operating system level requirements to run the application for the {os} operating system.
            2. Creates a new directory with the project name.
            3. Create all of the files and folders necessary to run the application that fulfills the application requirements.

            Script Guidelines:
            - If the bash script must be run in a specific directory, please include the necessary commands to change the directory.
            - Consolidate as many commands as possible into a single command.
            - If the bash command requires user input, please include the necessary commands to provide the input.
            - If the bash command requires restarting a terminal session, please include the necessary commands to restart the terminal session.
            - If the bash command requires a specific environment variable, please include the necessary commands to set the environment variable.

            The output must match the provided output json schema and be a valid json.

            Project Name:
            {name}

            Programming Language:
            {language}

            Application Requirements:
            {description}

            Output Json schema:
            {{
                \"script\": \"mkdir {name} && cd {name} && ...\"
            }}

            Respond ONLY with the data portion of a valid Json object. No schema definition required. No other words.",
            os=OS,
            name=args.name,
            language=args.language,
            description=args.description
    );
    println!("Sending prompt: {}", prompt);
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

    println!("Does this script look correct? `{script}`", script=contents.script);
    println!("Please input Y/N");
    let response: String = read!("{}\n");
    let cmd = {
        if response.replace(" ", "").to_uppercase() == "Y" {
            contents.script.clone()
        } else {
            println!("Damn robot, can you fix the command? Otherwise enter `skip` to go to the next step");
            let response: String = read!("{}\n");
            if response.contains("skip") {
                bail!("Robot failed to generate a valid script");
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

    // Print stdout and stderr from the child process
    child.stdout.as_mut().map(|stdout| {
        std::io::copy(stdout, &mut std::io::stdout()).unwrap();
    });
    child.stderr.as_mut().map(|stderr| {
        std::io::copy(stderr, &mut std::io::stderr()).unwrap();
    });

    let status = child.wait()?;
    if !status.success() {
        bail!(
            "Failed to execute `{}`. Exit code: {:?}",
            cmd,
            status.code()
        );
    }

    Ok(())
}

