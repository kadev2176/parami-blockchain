const parseError = (chain, error) => {
  if (error.isModule) {
    const decoded = chain.registry.findMetaError(error.asModule);
    return `error.${decoded.section}.${decoded.method}`;
  }

  return error.toString();
};

export const submit = (chain, call, payUser) => {
  return new Promise((resolve, reject) => {
    call.signAndSend(payUser, ({ events = [], status, dispatchError }) => {
      if (dispatchError) {
        reject(parseError(chain, dispatchError));
        return;
      }
      if (status.isInBlock) {
        // eslint-disable-next-line no-restricted-syntax
        for (const { data, method, section } of events) {
          if (section === 'magic' && method === 'Codo' && data[0].isError) {
            const error = data[0].asError;
            reject(parseError(chain, error));
            return;
          }
        }

        resolve(null);
      }
    });
  });
};
