
cases=`cat test_cases`
for c in $cases; do
  actual=`../../base9-builder render $c absolute.mustache`
  expected=${c:14:41}

  echo c, actual, expected
  echo $c
  echo $actual
  echo $expected
  echo ""
done
