import json
import os
from decimal import Decimal

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
        lang, name = get_condition(event)
        items = get_items(lang, name)
        body = {"Condition": {"language": lang, "string": name}, "Results": items}
        status = 200
    except BadRequestError as e:
        status = 400
        body["message"] = str(e)
    except (Exception, InternalServerError):
        pass
    return {
        "status": status,
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
        raise BadRequestError("language is required.")
    language = query.get("language")
    if language not in ["de", "en", "fr", "ja"]:
        raise BadRequestError(f'language "{language}" is invalid.')
    string = query.get("string")
    if not isinstance(string, str) or string == "":
        raise BadRequestError(f"string is required.")

    return language, string


def get_items(lang, string):
    resource = boto3.resource("dynamodb")
    table = resource.Table(os.environ["TABLE_NAME"])

    def scan(token=None):
        option = {"FilterExpression": Attr(f"Name_{lang}").contains(string)}
        if token is not None:
            option["ExclusiveStartKey"] = token
        resp = table.scan(**option)
        result = resp.get("Items", [])
        if "LastEvaluatedKey" in resp:
            result += scan(token=resp["LastEvaluatedKey"])
        return result

    items = scan()
    return list(sorted(items, key=lambda x: x["ItemSearchCategory"]["ID"]))
