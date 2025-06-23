variable "function_name" {
  description = "Name of the lambda function"
  type = string
}

variable "handler" {
  description = "Handler for the lambda function"
  type = string
  default = "bootstrap"
}

variable "runtime" {
  description = "Runtime for the lambda function"
  type = string
  default = "provided.al2023"
}

variable "filename" {
  description = "Filename of the lambda archive"
  type = string
}

variable "source_code_hash" {
  description = "Hash of the source for change detection"
  type = string
}

variable "environment" {
  type = map(string)
  default = {}
}

variable "aws_iam_policy_documents" {
  description = "List of JSON policy documents to attach"
  type = list(string)
  default = []
}