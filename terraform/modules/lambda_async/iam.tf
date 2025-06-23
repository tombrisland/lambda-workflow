resource "aws_iam_role" "this" {
  name = var.function_name

  assume_role_policy = jsonencode({
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "lambda.amazonaws.com"
        }
      }
    ]
  })
}

// Give permissions to input and output queues
data "aws_iam_policy_document" "this" {
  statement {
    actions = [
      "sqs:ReceiveMessage",
      "sqs:DeleteMessage",
      "sqs:GetQueueAttributes"
    ]
    resources = [
      aws_sqs_queue.input.arn
    ]
  }

  statement {
    actions = ["sqs:SendMessage"]
    resources = [
      aws_sqs_queue.output.arn,
      "*"
    ]
  }
}

resource "aws_iam_policy" "policies" {
  for_each = toset(concat([
    data.aws_iam_policy_document.this.json
    ],
    var.aws_iam_policy_documents
  ))

  policy = each.value
}

resource "aws_iam_role_policy_attachment" "this" {
  role       = aws_iam_role.this.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}