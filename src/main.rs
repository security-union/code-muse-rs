use async_openai::{
    types::{ChatCompletionRequestMessageArgs, CreateChatCompletionRequestArgs, Role},
    Client,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Describe what the program should do, be as specific as possible
    #[arg(short, long)]
    description: String,

    /// The name of the project which will be the name of the directory created
    #[arg(short, long, default_value = "myapp")]
    name: String,
}

#[derive(Deserialize, Serialize)]
struct OutputJson {
    dockerfile: String,
    makefile: String,
    source_files: Vec<SourceFile>,
    readme: String,
}

#[derive(Deserialize, Serialize)]
struct SourceFile {
    name: String,
    contents: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let prompt = format!(
            "Take the following programming language, application requirements, and produce a working application.

            your solution must include:
            1. Dockerfile that allows the application to be built and run
            2. Makefile that contains the following commands assuming that the application is executed using the Dockerfile.
                a. make build
                b. make run
                c. make test
            3. Readme with instructions required to build and run the application
            4. files with the source code for the application
            
            The output must match the provided output json schema and be a valid json.

            Project Name:
            {name}

            Programming Language:
            javascript

            Application Requirements:
            {description}

            Output Json schema:
            {{
                \"dockerfile\": \"dockerfile contents\",
                \"makefile\": \"makefile contents\",
                \"readme\": \"readme contents\",
                \"source_files\": [
                    {{
                        \"name\": \"...\",
                        \"contents\": \"...\"
                    }},
                    ...
                ]
            }}

            Respond ONLY with the data portion of a valid Json object. No schema definition required. No other words.",
            name=args.name,
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
    let contents: OutputJson = serde_json::from_str(&res.choices[0].message.content).map_err(|e| {
        println!("Failed to decode the contents, please try again. Sometimes OpenAI returns invalid JSON.");
        e
    })?;
    println!("Success, the robot has obeyed our orders.\n");

    println!("Generating the project files... 🤖");

    // Create a folder with the project name
    let project_name = args.name;
    let project_path = format!("./{}", project_name);
    println!("Creating project folder `{}`", project_path);
    Command::new("mkdir")
        .arg(project_path.clone())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;

    // Create a dockerfile
    let dockerfile_path = format!("{}/Dockerfile", project_path);
    println!("Creating dockerfile `{}`", dockerfile_path);
    std::fs::write(dockerfile_path.clone(), contents.dockerfile)?;

    // Create a makefile
    let makefile_path = format!("{}/Makefile", project_path);
    println!("Creating makefile `{}`", makefile_path);
    std::fs::write(makefile_path.clone(), contents.makefile)?;

    // Create a readme
    let readme_path = format!("{}/README.md", project_path);
    println!("Creating readme `{}`", readme_path);
    std::fs::write(readme_path.clone(), contents.readme)?;

    // Create source files
    let source_files_path = project_path;
    println!("Creating source files folder `{}`", source_files_path);
    // iterate through the source files and create them
    for source_file in contents.source_files {
        let source_file_path = format!("{}/{}", source_files_path, source_file.name);
        println!("Creating source file `{}`", source_file_path);
        std::fs::write(source_file_path.clone(), source_file.contents)?;
    }

    println!("Project files generated successfully ✅\n");
    println!("Disclaimer: This project was generated by a robot, please review the code before executing it.\n");
    println!("To execute the project, run the following commands:\n");
    println!("cd {}", project_name);
    println!("make build");
    println!("make run");
    // print disclaimer about the project

    Ok(())
}
