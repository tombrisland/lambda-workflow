resource "aws_lambda_function" "this" {
  function_name = var.function_name
  handler       = var.handler
  runtime       = var.runtime
  filename      = var.filename
  architectures = ["arm64"]

  role          = aws_iam_role.this.arn
  source_code_hash = var.source_code_hash

  environment {
    variables = merge({
      "SQS_WORKFLOW_INPUT_QUEUE_URL" = aws_sqs_queue.input.url
      "SQS_WORKFLOW_OUTPUT_QUEUE_URL" = aws_sqs_queue.output.url
    }, var.environment)
  }
}

resource "aws_lambda_event_source_mapping" "this" {
  function_name = aws_lambda_function.this.function_name

  // Don't retry
  maximum_retry_attempts = 1

  event_source_arn = aws_sqs_queue.input.arn
}

resource "aws_sqs_queue" "input" {
  name = "queue-${var.function_name}-in"
}

resource "aws_sqs_queue" "output" {
  name = "queue-${var.function_name}-out"
}