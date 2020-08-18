import json
import os
from typing import List, Optional

import boto3
from boto3.dynamodb.conditions import Attr


class BadRequestError(Exception):
    def __init__(self, message):
        self.message = message

    def __str__(self):
        return self.message


class InternalServerError(Exception):
    pass


def handler(event, context):
    response = {
        "status": 500,
        "headers": {
            "Content-Type": "application/json",
            "Access-Control-Allow-Origin": "*",
        },
    }
    body = {"type": "InternalServerError"}
    try:
        ids = get_condition(event)
        items = get_items(ids)
        body = {"Condition": {"ids": ids}, "Results": items}

        response["status"] = 200
    except BadRequestError as e:
        response["status"] = 400
        body["message"] = str(e)
    except (Exception, InternalServerError):
        pass
    response["body"] = json.dumps(body, default=str, ensure_ascii=False)
    return response


def get_condition(event):
    query = event.get("queryStringParameters")
    if query is None:
        raise BadRequestError("ids is required.")

    ids = query.get("ids")
    if not isinstance(ids, str) or ids == "":
        raise BadRequestError("ids is required.")

    try:
        return [int(x) for x in ids.split(",")]
    except Exception:
        raise BadRequestError("ids must be comma separated numbers")


def get_items(ids: List[int]):
    resource = boto3.resource("dynamodb")
    table_name = os.environ["TABLE_NAME"]

    def batch_get(keys: Optional[List[int]] = None, unprocessed=None):
        if [keys, unprocessed] == [None, None]:
            return []

        if len(keys) > 100:
            result = batch_get(keys=keys[:100])
            result += batch_get(keys=keys[100:])
            return result

        if keys is not None:
            option = {"RequestItems": {table_name: {"Keys": [{"ID": x} for x in keys]}}}
        elif unprocessed is not None:
            option = {"RequestItems": unprocessed}
        else:
            raise InternalServerError()

        resp = resource.batch_get_item(**option)
        result = resp.get("Responses", {}).get(table_name, [])
        if "UnprocessedKeys" in resp:
            result += batch_get(unprocessed=resp["UnprocessedKeys"])
        return result

    items = batch_get(keys=ids)
    return list(sorted(items, key=lambda x: x["ItemSearchCategory"]["ID"]))
