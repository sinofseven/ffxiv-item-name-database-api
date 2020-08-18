import json
import os
from decimal import Decimal
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
    status = 500
    body = {"type": "InternalServerError"}
    try:
        ids = get_condition(event)
        items = get_items(ids)
        body = {"Condition": {"ids": ids}, "Results": items}

        status = 200
    except BadRequestError as e:
        status = 400
        body["message"] = str(e)
    except (Exception, InternalServerError):
        pass
    return {
        "statusCode": status,
        "headers": {
            "Content-Type": "application/json",
            "Access-Control-Allow-Origin": "*",
        },
        "body": json.dumps(body, default=default, ensure_ascii=False),
    }


def default(obj):
    if isinstance(obj, Decimal):
        num = int(obj)
        return num if num == obj else float(obj)
    try:
        return str(obj)
    except Exception:
        return None


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
