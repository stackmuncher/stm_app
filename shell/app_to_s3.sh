#!/bin/bash -x
aws s3 cp target/release/stackmuncher s3://$STM_S3_BUCKET_PROD_BOOTSTRAP/apps/stackmuncher