
cases=`cat test_cases`
echo c, actual, expected
for c in $cases; do
  actual=`../../target/debug/base9-builder render $c ../../templates/absolute.mustache`
  expected=${c:14:41}

  echo $c
  echo $actual
  echo $expected
  echo ""
done
echo c, actual, expected
