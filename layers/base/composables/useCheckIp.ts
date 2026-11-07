export const useCheckIp = async () => {
  const ip = await useFetch("https://api.ipify.org?format=json").then((res) => {
    const data: any = res.data.value;
    const error = res.error.value;
    if (error) {
      console.log(error);
    } else {
      return data.ip;
    }
  });
  console.log(ip);
  return ip;
};
