
# Architecture

This service sends email if a bitcoin [lightning](https://lightning.network) invoice is paid.

The main use case is providing spam-free contact form for sites. 

This repository contains the code to run your own service but you can also access the feature by
using https://pay2.email which runs this code in the backend.

To run the service two components are necessary:

* The application server serving the endpoint to collect email information and present an invoice for sending an email.
* A [core lightning](https://github.com/ElementsProject/lightning) node with the 2 plugins contained in the [node-side](https://github.com/RCasatta/pay2email/tree/master/node-side) directory.
  * the `upload_invoices.py` plugin periodically poll the application server and upload fresh invoices when those are used or expired. It has been preferred to poll the application server for security reasons instead of letting the application server contacts the node directly
  * the `on_pay.py` plugin contacts the application server when an invoice is paid so that the relative email is sent


# Testing
```
PROTO=http
HOST=localhost:8000
USER=<USER>
PSW=<PSW>
I1=lnbc1n1psc9zuepp5wwtffxvvgpa3m2dx2gdaswur3r8lt0ga8khzk0s2mfa8p2zfmr9qdq9wdskwxqyjw5qcqpjsp5npsjwj9ca8htfzcgrlr9fw497yph9k99j38zn80h92vz8688297qrzjq2wjsl39dqxn3f0ppm388fckfgff6ka53canvg4m2wt5wx2xe5j46z46dvqq8gqqqqqqqqlgqqqqqqgq9q9qxsqyssqqwfj0nm99alenqjmpfny4rjnrn00x408x8t8vh2e2njq2eyl2qg8t8kjak6f3men482unrvghhsdp6v8yv8y2y2uakaqm3v809z29dgp4tyuyf
I2=lnbc1n1psc85zupp550d2ee5tlsutrq6nh0twp8g6p0pmquqk3kjml8s4edqpf73teluqdq8wdskwvsxqyjw5qcqpjsp5x6q0p577swhnjv0ungyc9h93smet3znreh5z0avh3f77ryaqk5mqrzjqvmkj3g9zgap9286mk24y0wvydvf3tfmszsxujnregn0a45d6rghczkqvsqqv4qqqqqqqqlgqqqqqqgqyg9qxsqyssqkrm6ek2dk7yvfd5x0c9k9w98uwuphkny7fj265tp0tj4fp0gekt3kctelvehy24n0ayrn5zqd2gwvzjwgy3r6dtgyjs8sunaa3fn3ucpc9l5md
I3=lnbc1n1psc8kqypp5s4teup8fr2mm4hj9vrvy99dhxt7ke4j0g767tl2wpzg8n2h9m79sdqgwdskwvnexqyjw5qcqpjsp5tj2d7kg0vynxccwpsw997etwm4su2d7ndysttn9aemaagzf86k4srzjqvfhr07eay6us6l4l6q5mrnvhj80u59yd4c37avr0gewxkmxf9q4xzdavyqqtlsqqqqqqq27qqqqqqgqjq9qxsqyssq52yuxmj8x0zpc8jgedae209tc2crv4qul8psvj2urkdvalgp63qjv53jevls20pj5dmvxk6xpwmgz8yc3kjnpvv3aakqfuy5j9mg3ygps229d8

curl $PROTO://$USER:$PSW@$HOST/invoice/all
curl -d $I1 $PROTO://$USER:$PSW@$HOST/invoice
curl -d $I2 $PROTO://$USER:$PSW@$HOST/invoice
curl -d $I3 $PROTO://$USER:$PSW@$HOST/invoice

# TO_PAY=$(curl -d 'from=riccardo.casatta@gmail.com&to=riccardo.casatta@gmail.com&subject=subject&message=ciao' localhost:8000/email | jq -r .bolt11)
# curl -d $TO_PAY http://$USER:$PSW@localhost:8000/invoice/paid

PROTO=https
HOST=pay2.email
```
