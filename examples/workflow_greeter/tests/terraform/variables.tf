variable "workflow_greeter_archive" {
  description = "Path to the built lambda workflow archive"
  default = "../../../../target/lambda/workflow_greeter/bootstrap.zip"
}

variable "workflow_greeter_handler" {
  description = "The lambda workflow handler to configure"
  // Bootstrap for rust lambdas
  default = "bootstrap"
}

variable "service_name_handler_file" {
  description = "Path to the handler for the name service"
  default = "../service_name/index.js"
}

variable "service_name_handler" {
  description = "The handler for the name service"
  default = "index.handler"
}