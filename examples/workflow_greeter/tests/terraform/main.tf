terraform {
  required_providers {
    aws = {
      source = "hashicorp/aws"
    }
  }
}

provider "aws" {
  region = "us-east-1"
}

// Table for use as a persistent state store
resource "aws_dynamodb_table" "state" {
  name = "state-store"

  hash_key  = "invocation_id"
  range_key = "task_id"

  attribute {
    name = "invocation_id"
    type = "S"
  }

  attribute {
    name = "task_id"
    type = "S"
  }

  billing_mode = "PAY_PER_REQUEST"
}

locals {
  service_name_archive = ".terraform/service_name.zip"
}

/**
  Example service lambda which generates a 'name' from a first_letter provided.
 */
module "lambda_name_service" {
  source        = "../../../../terraform/modules/lambda_async"
  function_name = "service_name"
  handler       = var.service_name_handler
  filename      = local.service_name_archive
  runtime       = "nodejs22.x"

  source_code_hash = data.archive_file.archive.output_md5
}

// Package up the single file service lambda
data "archive_file" "archive" {
  output_path = local.service_name_archive
  type        = "zip"

  source {
    content  = file(var.service_name_handler_file)
    filename = "index.js"
  }
}

/**
  The workflow implementation which calls the NodeJS impl.

  Ensure it's built via running `cargo lambda build -o zip --arm64`.
 */
module "lambda_greeter_workflow" {
  source        = "../../../../terraform/modules/lambda_async"
  function_name = "workflow_greeter"
  handler       = var.workflow_greeter_handler
  filename      = var.workflow_greeter_archive

  source_code_hash = filemd5(var.workflow_greeter_archive)

  environment = {
    SQS_NAME_SERVICE_QUEUE_URL = module.lambda_name_service.input_sqs_queue.url,
    AWS_LAMBDA_LOG_LEVEL = "debug"
  }
}
