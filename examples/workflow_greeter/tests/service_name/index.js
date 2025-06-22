const {SQSClient, SendMessageCommand} = require("@aws-sdk/client-sqs");

const client = new SQSClient();

/**
 * The example implementation for the name service.
 *
 * Each SQS message looks like this.
 * {
 *   "task_id": "abc1",
 *   "callback_url": "https://sqs.eu.amazon/queue-1",
 *   "payload": {
 *     "first_letter": "a"
 *   }
 * }
 *
 * @param event
 * @returns {Promise<void>}
 */
exports.handler = async (event) => {
    for (const record of event.Records) {
        const messageId = record.MessageId;
        const body = JSON.parse(record.body);

        console.log({messageId}, "Processing SQS record");

        const QueueUrl = body.callback_url;

        const {invocation_id, task_id, payload} = body;
        const {first_letter} = payload;

        const message = {
            QueueUrl,
            MessageBody: JSON.stringify({
                type: "Update",
                invocation_id,
                task_id,
                payload: {
                    first_letter,
                    name: `${first_letter}_generated_${Math.random()}`,
                }
            })
        };

        await client.send(new SendMessageCommand(message));

        console.log({messageId, task_id, QueueUrl}, "Sent response for task");
    }
}